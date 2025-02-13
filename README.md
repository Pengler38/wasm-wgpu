## Application build
`cargo run`

## Webserver build
First, install the wasm32 target. `rustup target install wasm32-unknown-unknown`
Second, `cargo install --locked trunk`
Then use `trunk serve --open` to build, start a webserver, and open a webpage to immediately test it
