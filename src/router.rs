use crate::route::{Path, PathSegment, Route};
use anyhow::Error;
use anyhow::Result;
use hyper::{
	body::Body,
	http::{response::Builder, Method},
	service::Service,
	Request, Response,
};
use std::{
	cmp::PartialEq,
	collections::HashMap,
	convert::Infallible,
	fmt::{self, Debug, Formatter},
	future::Future,
	pin::Pin,
	ptr,
	sync::Arc,
	task::{Context, Poll},
};

struct RouteNode<'a, T> {
	route: Option<Route<T>>,
	path: Option<RoutePath<'a, T>>,
}

impl<'a, T> Default for RouteNode<'a, T> {
	fn default() -> Self {
		Self {
			route: None,
			path: None,
		}
	}
}

impl<'a, T> PartialEq for RouteNode<'a, T> {
	fn eq(&self, other: &RouteNode<'a, T>) -> bool {
		ptr::eq(&self.route, &other.route) && self.path.eq(&other.path)
	}
}

impl<'a, T> Debug for RouteNode<'a, T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", &self)
	}
}

type RoutePath<'a, T> = HashMap<PathSegment<'a>, RouteNode<'a, T>>;
type Routes<'a, T> = HashMap<Method, RouteNode<'a, T>>;

pub type InternalErrorHandler = fn(e: Error) -> Response<Body>;
fn default_error_handler(e: Error) -> Response<Body> {
	Builder::default()
		.status(500)
		.body(e.to_string().into())
		.unwrap()
}

pub type NotFoundHandler = fn(req: Request<Body>) -> Response<Body>;
fn default_not_found_handler(_req: Request<Body>) -> Response<Body> {
	Builder::default().status(404).body(Body::empty()).unwrap()
}

#[derive(Debug)]
pub struct RouterBuilder<'a, T> {
	routes: Routes<'a, T>,
	pub internal_error_handler: Option<InternalErrorHandler>,
	pub not_found_handler: Option<NotFoundHandler>,
}

impl<'a, T> Default for RouterBuilder<'a, T> {
	fn default() -> Self {
		Self {
			routes: Routes::default(),
			internal_error_handler: None,
			not_found_handler: None,
		}
	}
}

impl<'a, T> RouterBuilder<'a, T> {
	pub fn register(&mut self, method: Method, path: Path<'a>, route: Route<T>) -> &mut Self {
		let mut node = self.routes.entry(method).or_default();

		let path_iter = path.into_iter();
		for segment in path_iter {
			node = node
				.path
				.get_or_insert(RoutePath::default())
				.entry(segment)
				.or_default();
		}

		node.route = Some(route);
		self
	}

	pub fn build(self) -> Router<'a, T> {
		Router {
			routes: Arc::new(self.routes),
			internal_error: self.internal_error_handler.unwrap_or(default_error_handler),
			not_found: self.not_found_handler.unwrap_or(default_not_found_handler),
		}
	}
}

#[derive(Debug)]
pub struct Router<'a, T> {
	routes: Arc<Routes<'a, T>>,
	internal_error: InternalErrorHandler,
	not_found: NotFoundHandler,
}

impl<T, U: 'static> Service<T> for Router<'static, U> {
	type Response = RouteHandler<'static, U>;
	type Error = Infallible;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _: T) -> Self::Future {
		let routes = Arc::clone(&self.routes);
		let internal_error = self.internal_error;
		let not_found = self.not_found;

		let fut = async move {
			Ok(RouteHandler {
				routes,
				internal_error,
				not_found,
			})
		};
		Box::pin(fut)
	}
}

pub struct RouteHandler<'a, T> {
	routes: Arc<Routes<'a, T>>,
	internal_error: InternalErrorHandler,
	not_found: NotFoundHandler,
}

impl<'a, T> Service<Request<Body>> for RouteHandler<'a, T>
where
	T: 'static + Future<Output = Result<hyper::Response<Body>>> + Send,
{
	type Response = Response<Body>;
	type Error = Infallible;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		let mut maybe_node = self.routes.get(req.method());

		let uri = req.uri().clone();
		let path = uri.path().strip_prefix('/').unwrap_or_default().split('/');
		let mut params = vec![];

		for segment in path {
			if segment.is_empty() {
				continue;
			}

			match maybe_node {
				None => break,
				Some(node) => {
					maybe_node = node.path.as_ref().and_then(|routes| {
						routes.get(&PathSegment::Static(segment)).or_else(|| {
							params.push(segment.to_owned());
							routes.get(&PathSegment::Dynamic)
						})
					})
				}
			}
		}

		match maybe_node.and_then(|node| node.route) {
			Some(route) => {
				let fut = route(params, req);
				let err = self.internal_error;
				Box::pin(async move { Ok(fut.await.unwrap_or_else(err)) })
			}
			None => {
				let response = (self.not_found)(req);
				Box::pin(async { Ok(response) })
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::{RouteNode, RoutePath, RouterBuilder};
	use crate::{path, PathSegment, Request, Response};
	use hyper::{http::response, Body, Method};

	async fn test_route(_params: Vec<String>, _req: Request) -> Response {
		Ok(response::Builder::default()
			.status(200)
			.body(Body::empty())?)
	}

	trait Apply<F> {
		fn apply(self, applicator: F) -> Self;
	}

	impl<T, F> Apply<F> for T
	where
		F: FnOnce(&mut Self) -> (),
	{
		fn apply(mut self, applicator: F) -> Self {
			applicator(&mut self);
			self
		}
	}

	#[test]
	fn adds_routes() {
		let mut builder = RouterBuilder::default();
		builder.register(Method::GET, path![], test_route);
		builder.register(Method::POST, path![_], test_route);
		builder.register(Method::PUT, path![_ / foo / bar], test_route);

		assert_eq!(
			builder.routes.get(&Method::GET),
			Some(&RouteNode {
				route: Some(test_route),
				path: None,
			})
		);

		assert_eq!(
			builder.routes.get(&Method::POST),
			Some(&RouteNode {
				route: None,
				path: Some(RoutePath::new().apply(|path| {
					path.insert(
						PathSegment::Dynamic,
						RouteNode {
							route: Some(test_route),
							path: None,
						},
					);
				}))
			})
		);

		assert_eq!(
			builder.routes.get(&Method::PUT),
			Some(&RouteNode {
				route: None,
				path: Some(RoutePath::new().apply(|path| {
					path.insert(
						PathSegment::Dynamic,
						RouteNode {
							route: None,
							path: Some(RoutePath::new().apply(|path| {
								path.insert(
									PathSegment::Static("foo"),
									RouteNode {
										route: None,
										path: Some(RoutePath::new().apply(|path| {
											path.insert(
												PathSegment::Static("bar"),
												RouteNode {
													route: Some(test_route),
													path: None,
												},
											);
										})),
									},
								);
							})),
						},
					);
				})),
			})
		);
	}
}
