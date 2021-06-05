# Nestadia WASM
This version of Nestadia runs directly in the browser!

## How to run
To start off, install `trunk` and `wasm-bindgen-cli` via cargo:
```
cargo install trunk wasm-bindgen-cli
```
After that, simply use trunk to build and serve the application. Note that performance are very bad in debug builds:
```
trunk serve --release
```
After that the application will be exposed on `http://localhost:8080`