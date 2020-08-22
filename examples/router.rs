use grout::{response::Builder, Body, PathSegment, Request, Response, RouterBuilder};
use hyper::{Method, Server};

fn handle_get(_req: Request) -> Response {
	let res = Builder::default();
	Box::pin(async move { Ok(res.body(Body::empty())?) })
}

fn handle_post(_req: Request) -> Response {
	let res = Builder::default();
	Box::pin(async move { Ok(res.body(Body::empty())?) })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let addr = ([127, 0, 0, 1], 3000).into();

	let mut builder = RouterBuilder::default();
	builder.register(Method::GET, vec![], handle_get).register(
		Method::POST,
		vec![PathSegment::Static("foo")],
		handle_post,
	);

	let router = builder.build();

	let server = Server::bind(&addr).serve(router);
	println!("Listening on http://{}", addr);

	server.await?;
	Ok(())
}
