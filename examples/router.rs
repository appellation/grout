use grout::{
	hyper::{Body, Method, Server},
	path, HttpRouter, PathSegment, Request, Response, ResponseBuilder, Router,
};

async fn handler(params: Vec<String>, _req: Request) -> Response {
	let res = ResponseBuilder::default();
	dbg!(params);
	Ok(res.body(Body::empty())?)
}

async fn other_handler(_params: Vec<String>, _req: Request) -> Response {
	let res = ResponseBuilder::default();
	Ok(res.body(Body::empty())?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let addr = ([127, 0, 0, 1], 3000).into();

	let builder = Router::default()
		.register(Method::GET, path![], handler)
		.register(Method::POST, path![foo / _ / bar / _ / baz], handler)
		.register(Method::GET, path![_], other_handler);

	let router = HttpRouter::from(builder);

	let server = Server::bind(&addr).serve(router);
	println!("Listening on http://{}", addr);

	server.await?;
	Ok(())
}
