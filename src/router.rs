use crate::route::{DynRoute, Path, PathSegment, Route};
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

#[derive(Default)]
struct RouteNode<'a> {
	route: Option<DynRoute>,
	path: Option<RoutePath<'a>>,
}

impl<'a> PartialEq for RouteNode<'a> {
	fn eq(&self, other: &RouteNode<'a>) -> bool {
		ptr::eq(&self.route, &other.route) && self.path.eq(&other.path)
	}
}

impl<'a> Debug for RouteNode<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", &self)
	}
}

type RoutePath<'a> = HashMap<PathSegment<'a>, RouteNode<'a>>;
type Routes<'a> = HashMap<Method, RouteNode<'a>>;

/// A function that can convert an error into a response.
pub type InternalErrorHandler = fn(e: Error) -> Response<Body>;
fn default_error_handler(e: Error) -> Response<Body> {
	Builder::default()
		.status(500)
		.body(e.to_string().into())
		.unwrap()
}

/// A function that handles unroutable requests and creates a response.
pub type NotFoundHandler = fn(req: Request<Body>) -> Response<Body>;
fn default_not_found_handler(_req: Request<Body>) -> Response<Body> {
	Builder::default().status(404).body(Body::empty()).unwrap()
}

/// A struct to simplify the construction of the router service. Enables registration of
/// routes and handlers before instantiating the router.
#[derive(Debug)]
pub struct RouterBuilder<'a> {
	routes: Routes<'a>,
	pub internal_error_handler: Option<InternalErrorHandler>,
	pub not_found_handler: Option<NotFoundHandler>,
}

impl<'a> Default for RouterBuilder<'a> {
	fn default() -> Self {
		Self {
			routes: Routes::default(),
			internal_error_handler: None,
			not_found_handler: None,
		}
	}
}

impl<'a> RouterBuilder<'a> {
	pub fn register<T: 'static + Future<Output = Result<Response<Body>>> + Send>(
		mut self,
		method: Method,
		path: Path<'a>,
		route: Route<T>,
	) -> Self {
		let mut node = self.routes.entry(method).or_default();

		let path_iter = path.into_iter();
		for segment in path_iter {
			node = node
				.path
				.get_or_insert(RoutePath::default())
				.entry(segment)
				.or_default();
		}
		node.route = Some(Box::new(move |params: Vec<String>, req: Request<Body>| {
			Box::pin(route(params, req))
		}));
		self
	}

	pub fn build(self) -> Router<'a> {
		Router {
			routes: Arc::new(self.routes),
			internal_error: self.internal_error_handler.unwrap_or(default_error_handler),
			not_found: self.not_found_handler.unwrap_or(default_not_found_handler),
		}
	}
}

/// Intended to be used as the main service with hyper.
/// ```
/// #[tokio::main]
/// fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// 	let addr = ([127, 0, 0, 1], 3000).into();
/// 	let server = Server::bind(&addr).serve(RouteBuilder::default().build());
/// 	server.await?;
/// 	Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Router<'a> {
	routes: Arc<Routes<'a>>,
	internal_error: InternalErrorHandler,
	not_found: NotFoundHandler,
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

/// Responsible for handling the actual HTTP requests from hyper.
pub struct RouteHandler<'a> {
	routes: Arc<Routes<'a>>,
	internal_error: InternalErrorHandler,
	not_found: NotFoundHandler,
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
							let route = routes.get(&PathSegment::Dynamic);
							if route.is_some() {
								params.push(segment.to_owned());
							}

							route
						})
					})
				}
			}
		}

		match maybe_node.and_then(|node| node.route.as_ref()) {
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
