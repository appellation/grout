use anyhow::Result;
use hyper::Body;
use std::{future::Future, pin::Pin};

pub type Request = hyper::Request<Body>;
pub type Response = Result<hyper::Response<Body>>;

/// A route path is just a vec of [PathSegment](enum.PathSegment.html)s.
///
/// Use the [path!](../macro.path.html) macro to generate this more easily.
pub type Path<'a> = Vec<PathSegment<'a>>;

/// Create a [Path](route/type.Path.html) with simplified syntax.
/// ```
/// path![foo / _ / bar / _] // -> vec![Static("foo"), Dynamic, Static("bar"), Dynamic]
/// ```
#[macro_export]
macro_rules! path {
	[] => { vec![] };
	[ @single _ ] => {
		PathSegment::Dynamic
	};
	[ @single $first:tt ] => {
		PathSegment::Static(stringify!($first))
	};
	[ $($segment:tt) / * ] => {
		vec![$(path![@single $segment]), *]
	};
}

/// Path segments are matched during routing. Static segments are matched through hash equality.
/// If no static segments match, a corresponding dynamic segment is attempted. For example:
/// `GET /foo/bar` matches `vec![Static("foo"), Dynamic]` instead of `vec![Dynamic, Dynamic]`.
///
/// Dynamic parameters are collected during routing and passed into the handler in an ordered list.
#[derive(Debug, Eq, PartialEq, Hash)]
pub enum PathSegment<'a> {
	Dynamic,
	Static(&'a str),
}

/// Represents the route handler type. Although this is typed with a generic return type, this is
/// only to allow async functions to be used as handlers. T is generally going to be `impl Future<
/// Output = Response>`, meaning your route handlers are going to look exactly like this:
/// ```
/// async fn handler(params: Vec<String>, req: Request) -> Response {}
/// ```
pub type Route<T> = fn(Vec<String>, Request) -> T;

/// Boxed closure for route handlers. Apparently different abstract types don't match, so we need
/// to box the return type of the user-land route handlers. To keep the API clean, this type is
/// used internally and created when the user registers a route.
pub(crate) type DynRoute = Box<
	dyn Fn(Vec<String>, Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync,
>;
