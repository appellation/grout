use crate::route::{DynRoute, Path, PathSegment, Route};
use std::{
	cmp::PartialEq,
	collections::HashMap,
	fmt::{self, Debug, Formatter},
	future::Future,
	hash::Hash,
	ptr,
};

pub struct RouteNode<'path, Req, Res> {
	pub route: Option<DynRoute<Req, Res>>,
	pub path: Option<RoutePath<'path, Req, Res>>,
}

impl<'path, Req, Res> Default for RouteNode<'path, Req, Res> {
	fn default() -> Self {
		Self {
			route: None,
			path: None,
		}
	}
}

impl<'path, Req, Res> PartialEq for RouteNode<'path, Req, Res> {
	fn eq(&self, other: &RouteNode<'path, Req, Res>) -> bool {
		ptr::eq(&self.route, &other.route) && self.path.eq(&other.path)
	}
}

impl<'a, Req, Res> Debug for RouteNode<'a, Req, Res> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", &self)
	}
}

type RoutePath<'path, Req, Res> = HashMap<PathSegment<'path>, RouteNode<'path, Req, Res>>;
pub type Routes<'path, Prefix, Req, Res> = HashMap<Prefix, RouteNode<'path, Req, Res>>;

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
pub struct Router<'a, Prefix, Req, Res> {
	routes: Routes<'a, Prefix, Req, Res>,
}

impl<'a, Prefix, Req, Res> Default for Router<'a, Prefix, Req, Res> {
	fn default() -> Self {
		Self {
			routes: Default::default(),
		}
	}
}

impl<'a, Prefix, Req, Res> Router<'a, Prefix, Req, Res>
where
	Req: 'static,
	Prefix: Eq + Hash,
{
	pub fn register<T: 'static + Future<Output = Res> + Send>(
		mut self,
		prefix: Prefix,
		path: Path<'a>,
		route: Route<Req, T>,
	) -> Self {
		let mut node = self.routes.entry(prefix).or_default();

		let path_iter = path.into_iter();
		for segment in path_iter {
			node = node
				.path
				.get_or_insert(RoutePath::default())
				.entry(segment)
				.or_default();
		}
		node.route = Some(Box::new(move |params: Vec<String>, req: Req| {
			Box::pin(route(params, req))
		}));
		self
	}

	pub fn find_node<'path>(
		&self,
		prefix: &Prefix,
		path: &'path str,
	) -> (Vec<String>, Option<&'path RouteNode<Req, Res>>) {
		path.strip_prefix('/')
			.unwrap_or_default()
			.split('/')
			.filter(|s| !s.is_empty())
			.try_fold(
				(vec![], self.routes.get(prefix)),
				|(mut params, maybe_node), segment| match maybe_node {
					None => Err((params, maybe_node)),
					Some(node) => {
						let new_node = node.path.as_ref().and_then(|routes| {
							routes.get(&PathSegment::Static(segment)).or_else(|| {
								let route = routes.get(&PathSegment::Dynamic);
								if route.is_some() {
									params.push(segment.to_owned());
								}

								route
							})
						});

						Ok((params, new_node))
					}
				},
			)
			.unwrap_or_else(|e| e)
	}
}
