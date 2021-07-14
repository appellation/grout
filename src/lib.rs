//! A dead simple hash-based HTTP router built on hyper.
//!
//! ```
//! use grout::{path, Body, Method, Request, Response, ResponseBuilder, RouterBuilder, Server};
//!
//! async fn handler(params: Vec<String>, _req: Request) -> Response {
//! 	Ok(ResponseBuilder::default().body(Body::empty())?)
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! 	let addr = ([127, 0, 0, 1], 3000).into();
//! 	let router = RouterBuilder::default()
//! 		.register(Method::GET, path![], handler)
//! 		.register(Method::GET, path![foo / _], handler)
//! 		.register(Method::POST, path![foo / _], handler)
//! 		.build();
//!
//! 	let server = Server::bind(&addr).serve(router);
//! 	println!("Listening on http://{}", addr);
//!
//! 	server.await?;
//! 	Ok(())
//! }
//! ```
//!
//! Path segments denoted with a `_` are matched dynamically if no other static segment matches.
//! Dynamic segments are passed into the route handler as the first parameter. Only one route can
//! match any given request.
//!
//! The router builder exposes `internal_error_handler` and `not_found_handler` which can handle
//! errors returned from handlers and unmatched requests respectively.

#[cfg(feature = "http")]
mod http;
#[cfg(feature = "http")]
pub use http::*;

// mod pool;

/// Various types and utilities for defining routes and route handlers.
pub mod route;

/// Contains the core structs of the router.
///
/// Use the RouterBuilder to create a Router: pass the router to hyper as the service.
pub mod router;

pub use route::*;
pub use router::*;
