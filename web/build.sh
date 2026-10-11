#!/usr/bin/env sh
# Builds the WebAssembly package the page loads into web/pkg.
# Serve the result with any static file server, e.g.
#   python3 -m http.server --directory web 8000
set -eu
cd "$(dirname "$0")/.."
wasm-pack build crates/corewarz-web --target web --release --no-typescript --out-dir ../../web/pkg
