use crate::route::{Path, PathSegment, Route};
use hyper::{
	body::Body,
	http::{response::Builder, Method},
	service::Service,
	Request, Response,
};
use std::{
	collections::HashMap,
	convert::Infallible,
	future::Future,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};

#[derive(Debug, Default, Eq, PartialEq)]
struct RouteNode<'a> {
	route: Option<Route>,
	path: Option<RoutePath<'a>>,
}

type RoutePath<'a> = HashMap<PathSegment<'a>, RouteNode<'a>>;
type Routes<'a> = HashMap<Method, RouteNode<'a>>;

#[derive(Debug, Default)]
pub struct RouterBuilder<'a> {
	routes: Routes<'a>,
}

impl<'a> RouterBuilder<'a> {
	pub fn register(&mut self, method: Method, path: Path<'a>, route: Route) -> &mut Self {
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

	pub fn build(self) -> Router<'a> {
		Router {
			routes: Arc::new(self.routes),
		}
	}
}

#[derive(Debug)]
pub struct Router<'a> {
	routes: Arc<Routes<'a>>,
}

impl<T> Service<T> for Router<'static> {
	type Response = RouteHandler<'static>;
	type Error = Infallible;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _: T) -> Self::Future {
		let routes = Arc::clone(&self.routes);
		let fut = async move { Ok(RouteHandler { routes }) };
		Box::pin(fut)
	}
}

pub struct RouteHandler<'a> {
	routes: Arc<Routes<'a>>,
}

impl<'a> Service<Request<Body>> for RouteHandler<'a> {
	type Response = Response<Body>;
	type Error = Infallible;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		let mut maybe_node = self.routes.get(req.method());

		let uri = req.uri().clone();
		dbg!(&uri);
		let path = uri.path().strip_prefix('/').unwrap_or_default().split('/');
		dbg!(&maybe_node);

		for segment in path {
			if segment.is_empty() {
				continue;
			}

			match maybe_node {
				None => break,
				Some(node) => {
					maybe_node = node.path.as_ref().and_then(|routes| {
						routes
							.get(&PathSegment::Static(segment))
							.or_else(|| routes.get(&PathSegment::Dynamic))
					})
				}
			}
		}

		dbg!(&maybe_node);
		match maybe_node.and_then(|node| node.route) {
			Some(route) => {
				let fut = route(req);
				Box::pin(async move {
					Ok(fut.await.unwrap_or_else(|e| {
						Builder::default()
							.status(500)
							.body(e.to_string().into())
							.unwrap()
					}))
				})
			}
			None => {
				Box::pin(async { Ok(Builder::default().status(404).body(Body::empty()).unwrap()) })
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::{RouterBuilder, RouteNode, RoutePath};
	use crate::{Request, Response, response, Body, PathSegment, Method};

	fn test_route(_req: Request) -> Response {
		Box::pin(async move {
			Ok(response::Builder::default().status(200).body(Body::empty())?)
		})
	}

	trait Apply<F> {
		fn apply(self, applicator: F) -> Self;
	}

	impl<T, F> Apply<F> for T
	where F: FnOnce(&mut Self) -> () {
		fn apply(mut self, applicator: F) -> Self {
			applicator(&mut self);
			self
		}
	}

	#[test]
	fn adds_routes() {
		let mut builder = RouterBuilder::default();
		builder.register(Method::GET, vec![], test_route);
		builder.register(Method::POST, vec![PathSegment::Dynamic], test_route);
		builder.register(Method::PUT, vec![PathSegment::Dynamic, PathSegment::Static("foo"), PathSegment::Static("bar")], test_route);

		assert_eq!(builder.routes.get(&Method::GET), Some(&RouteNode {
			route: Some(test_route),
			path: None,
		}));

		assert_eq!(builder.routes.get(&Method::POST), Some(&RouteNode {
			route: None,
			path: Some(RoutePath::new().apply(|path| {
				path.insert(PathSegment::Dynamic, RouteNode {
					route: Some(test_route),
					path: None,
				});
			}))
		}));

		let mut put_route = RoutePath::new();
		put_route.insert(PathSegment::Dynamic, RouteNode {
			route: None,
			path: Some(RoutePath::new())
		});

		assert_eq!(builder.routes.get(&Method::PUT), Some(&RouteNode {
			route: None,
			path: Some(RoutePath::new().apply(|path| {
				path.insert(PathSegment::Dynamic, RouteNode {
					route: None,
					path: Some(RoutePath::new().apply(|path| {
						path.insert(PathSegment::Static("foo"), RouteNode {
							route: None,
							path: Some(RoutePath::new().apply(|path| {
								path.insert(PathSegment::Static("bar"), RouteNode {
									route: Some(test_route),
									path: None,
								});
							})),
						});
					})),
				});
			})),
		}));
	}
}
