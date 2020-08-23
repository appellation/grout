use anyhow::Result;
use hyper::Body;
use std::{future::Future, pin::Pin};

pub type Request = hyper::Request<Body>;
pub type Response = Pin<Box<dyn Future<Output = Result<hyper::Response<Body>>> + Send>>;
pub type Path<'a> = Vec<PathSegment<'a>>;

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

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum PathSegment<'a> {
	Dynamic,
	Static(&'a str),
}

pub type Route = fn(Vec<String>, Request) -> Response;
