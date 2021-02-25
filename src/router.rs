use crate::route::{Path, PathSegment, Route};
use std::{
	cmp::PartialEq,
	collections::HashMap,
	fmt::{self, Debug, Formatter},
	hash::Hash,
	ptr,
};

type RoutePath<T, R> = HashMap<PathSegment, RouteNode<T, R>>;
type Routes<P, T, R> = HashMap<P, RouteNode<T, R>>;

pub struct RouteNode<T, R> {
	pub(crate) route: Option<Box<dyn Route<T, R>>>,
	path: Option<RoutePath<T, R>>,
}

impl<T, R> PartialEq for RouteNode<T, R> {
	fn eq(&self, other: &RouteNode<T, R>) -> bool {
		ptr::eq(&self.route, &other.route) && self.path.eq(&other.path)
	}
}

impl<T, R> Debug for RouteNode<T, R> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", &self)
	}
}

impl<T, R> Default for RouteNode<T, R> {
	fn default() -> Self {
		Self {
			route: None,
			path: None,
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
pub struct Router<P, T, R> {
	routes: Routes<P, T, R>,
}

impl<P, T, R> Default for Router<P, T, R> {
	fn default() -> Self {
		Self {
			routes: Default::default(),
		}
	}
}

impl<P, T, R> Router<P, T, R>
where
	P: Eq + Hash,
{
	pub fn register(
		&mut self,
		method: P,
		path: Path,
		route: impl Route<T, R> + 'static,
	) -> &mut Self {
		let mut node = self.routes.entry(method).or_default();

		for segment in path.into_iter() {
			node = node
				.path
				.get_or_insert(RoutePath::default())
				.entry(segment)
				.or_default();
		}
		node.route = Some(Box::new(route));
		self
	}

	pub fn find_node<'path>(
		&self,
		prefix: &P,
		path: &'path str,
	) -> (Vec<&'path str>, Option<&RouteNode<T, R>>) {
		path.strip_prefix('/')
			.unwrap_or_default()
			.split('/')
			.filter(|segment| !segment.is_empty())
			.try_fold(
				(vec![], self.routes.get(prefix)),
				|(mut params, maybe_node), segment| match maybe_node {
					None => Err((params, maybe_node)),
					Some(node) => {
						let new_node = node.path.as_ref().and_then(|routes| {
							routes
								.get(&PathSegment::Static(segment.to_owned().into()))
								.or_else(|| {
									let route = routes.get(&PathSegment::Dynamic);
									if route.is_some() {
										params.push(segment);
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
