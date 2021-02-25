use std::{borrow::Cow, future::Future, pin::Pin};

/// A route path is just a vec of [PathSegment](enum.PathSegment.html)s.
///
/// Use the [path!](../macro.path.html) macro to generate this more easily.
pub type Path = Vec<PathSegment>;

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
		PathSegment::Static(stringify!($first).into())
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
pub enum PathSegment {
	Dynamic,
	Static(Cow<'static, str>),
}

pub trait Route<T, R> {
	fn route(&self, params: Vec<&str>, body: &T) -> R;
}

// impl<F, T, R> Route<T, R> for F
// where
// 	F: Fn(Vec<&str>, &T) -> R,
// {
// 	fn route(&self, params: Vec<&str>, body: &T) -> R {
// 		(self)(params, body)
// 	}
// }

impl<F, Fut, T, R> Route<T, Pin<Box<dyn Future<Output = R> + Send + 'static>>> for F
where
	F: Fn(Vec<&str>, &T) -> Fut,
	Fut: Future<Output = R>,
{
	fn route(&self, _: Vec<&str>, _: &T) -> Pin<Box<dyn Future<Output = R> + Send + 'static>> {
		todo!()
	}
}
