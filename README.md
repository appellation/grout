# grout

A dead simple hash-based HTTP router built on hyper.

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
