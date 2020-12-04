# grout

A dead simple hash-based HTTP router built on hyper.

```rust
use grout::{
	hyper::{Body, Method, Server},
	path, PathSegment, Request, Response, ResponseBuilder, RouterBuilder,
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

	let builder = RouterBuilder::default()
		.register(Method::GET, path![], handler)
		.register(Method::POST, path![foo / _ / bar / _ / baz], handler)
		.register(Method::GET, path![_], other_handler);

	let router = builder.build();

	let server = Server::bind(&addr).serve(router);
	println!("Listening on http://{}", addr);

	server.await?;
	Ok(())
}

```

## Features

- HTTP method routing
- Route parameters (ordered, not keyed)
- Simple API

See the examples folder for example usage.

## Limitations

- No state passing or any form of middleware
	- I recommend the [`state`](https://github.com/SergioBenitez/state) crate to inject outside
		structs into your route handlers
- No complex route matching
	- Perform complex validation in your route handlers
