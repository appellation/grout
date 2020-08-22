use grout::{response::Builder, Body, PathSegment, Request, Response, RouterBuilder};
use hyper::{Method, Server};

fn handle_get(params: Vec<String>, _req: Request) -> Response {
	let res = Builder::default();
	dbg!(params);
	Box::pin(async move { Ok(res.body(Body::empty())?) })
}

fn handle_post(_params: Vec<String>, _req: Request) -> Response {
	let res = Builder::default();
	Box::pin(async move { Ok(res.body(Body::empty())?) })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let addr = ([127, 0, 0, 1], 3000).into();

	let mut builder = RouterBuilder::default();
	builder
		.register(Method::GET, vec![], handle_get)
		.register(Method::POST, vec![PathSegment::Static("foo")], handle_post)
		.register(Method::GET, vec![PathSegment::Dynamic], handle_get);

	let router = builder.build();

	let server = Server::bind(&addr).serve(router);
	println!("Listening on http://{}", addr);

	server.await?;
	Ok(())
}
