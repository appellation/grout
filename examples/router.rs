use grout::{response::Builder, Body, path, PathSegment, Request, Response, RouterBuilder};
use hyper::{Method, Server};

fn handler(params: Vec<String>, _req: Request) -> Response {
	let res = Builder::default();
	dbg!(params);
	Box::pin(async move { Ok(res.body(Body::empty())?) })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let addr = ([127, 0, 0, 1], 3000).into();

	let mut builder = RouterBuilder::default();
	builder
		.register(Method::GET, path![], handler)
		.register(Method::POST, path![foo / _ / bar / _ / baz], handler)
		.register(Method::GET, path![_], handler);

	let router = builder.build();

	let server = Server::bind(&addr).serve(router);
	println!("Listening on http://{}", addr);

	server.await?;
	Ok(())
}
