use crate::Router;
use anyhow::{Error, Result};
use hyper::{
	body::Body,
	http::{response::Builder, Method},
	service::Service,
};
use std::{
	convert::Infallible,
	future::{ready, Future, Ready},
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};

pub use hyper;

pub use hyper::http::response::Builder as ResponseBuilder;
pub type Request = hyper::Request<Body>;
pub type Response = Result<hyper::Response<Body>>;

fn default_error_handler(e: Error) -> hyper::Response<Body> {
	Builder::default()
		.status(500)
		.body(e.to_string().into())
		.unwrap()
}

fn default_not_found_handler(_req: Request) -> hyper::Response<Body> {
	Builder::default().status(404).body(Body::empty()).unwrap()
}

/// A function that can convert an error into a response.
pub type ErrorHandler = fn(e: Error) -> hyper::Response<Body>;

/// A function that handles unroutable requests and creates a response.
pub type NotFoundHandler = fn(req: Request) -> hyper::Response<Body>;

type InnerHttpRouter<'a> = Router<'a, Method, Request, Response>;

pub struct HttpRouter {
	router: Arc<InnerHttpRouter<'static>>,
	internal_error: ErrorHandler,
	not_found: NotFoundHandler,
}

impl From<InnerHttpRouter<'static>> for HttpRouter {
	fn from(inner: InnerHttpRouter<'static>) -> Self {
		Self {
			router: Arc::new(inner),
			internal_error: default_error_handler,
			not_found: default_not_found_handler,
		}
	}
}

impl<T> Service<T> for HttpRouter {
	type Response = RouteHandler<'static>;
	type Error = Infallible;
	type Future = Ready<Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _: T) -> Self::Future {
		let router = Arc::clone(&self.router);
		let internal_error = self.internal_error.clone();
		let not_found = self.not_found.clone();

		ready(Ok(RouteHandler {
			router,
			internal_error,
			not_found,
		}))
	}
}

/// Responsible for handling the actual HTTP requests from hyper.
pub struct RouteHandler<'a> {
	router: Arc<InnerHttpRouter<'a>>,
	internal_error: ErrorHandler,
	not_found: NotFoundHandler,
}

impl<'a> Service<Request> for RouteHandler<'a> {
	type Response = hyper::Response<Body>;
	type Error = Infallible;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request) -> Self::Future {
		let uri = req.uri().clone();
		let (params, maybe_node) = self.router.find_node(req.method(), uri.path());

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
