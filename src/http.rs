use std::{convert::Infallible, future::{self, Future}, ops::Deref, pin::Pin, sync::Arc, task::{Context, Poll}};

use anyhow::{Error, Result};
use hyper::{service::Service, Body, Method};

use crate::{Request, Response, Route, Router};

type InnerHttpResponse<O = Response> = Pin<Box<dyn Future<Output = O> + Send>>;
type InnerHttpRouter = Arc<Router<Method, Request, InnerHttpResponse>>;

pub struct HttpRouter<E, N> {
	inner: InnerHttpRouter,
	pub error_handler: Option<E>,
	pub not_found_handler: Option<N>,
}

impl<E, N> Default for HttpRouter<E, N> {
	fn default() -> Self {
		Self {
			inner: Default::default(),
			error_handler: None,
			not_found_handler: None,
		}
	}
}

impl<E, N> Deref for HttpRouter<E, N> {
	type Target = InnerHttpRouter;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

// /// A function that can convert an error into a response.
// pub type InternalErrorHandler = fn(e: Error) -> Response<Body>;
async fn default_error_handler(e: Error) -> hyper::Response<Body> {
	todo!()
	// Builder::default()
	// 	.status(500)
	// 	.body(e.to_string().into())
	// 	.unwrap()
}

// /// A function that handles unroutable requests and creates a response.
// pub type NotFoundHandler = fn(req: Request<Body>) -> Response<Body>;
async fn default_not_found_handler(_req: &Request) -> Response {
	todo!()
	// Builder::default().status(404).body(Body::empty()).unwrap()
}

impl<T, E: Clone + 'static, N: Clone + 'static> Service<T> for HttpRouter<E, N> {
	type Response = RouteHandler<E, N>;
	type Error = Infallible;
	type Future = future::Ready<Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _: T) -> Self::Future {
		let router = Arc::clone(&self.inner);
		let error_handler = self.error_handler.clone();
		let not_found_handler = self.not_found_handler.clone();

		future::ready(
			Ok(RouteHandler {
				inner: router,
				error_handler,
				not_found_handler,
			})
		)
	}
}

/// Responsible for handling the actual HTTP requests from hyper.
pub struct RouteHandler<E, N> {
	inner: InnerHttpRouter,
	error_handler: Option<E>,
	not_found_handler: Option<N>,
}

impl<E, N> Service<Request> for RouteHandler<E, N>
where
	E: Fn(Error) -> hyper::Response<Body> + Clone + Send + Sync + 'static,
	N: Route<Request, InnerHttpResponse>,
{
	type Response = hyper::Response<Body>;
	type Error = Infallible;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request) -> Self::Future {
		let (params, maybe_node) = self.inner.find_node(req.method(), req.uri().path());

		let response = match maybe_node.and_then(|node| node.route.as_ref()) {
			Some(route) => route.route(params, &req),
			None => match &self.not_found_handler {
				Some(handler) => handler.route(params, &req),
				None => todo!(), // None => Box::pin(default_not_found_handler(&req)),
			},
		};

		let error_handler = self.error_handler.clone();
		Box::pin(async move {
			match response.await {
				Ok(res) => Ok(res),
				Err(e) => Ok(match &error_handler {
					Some(handler) => (handler)(e),
					None => default_error_handler(e).await,
				}),
			}
		})
	}
}
