#!/bin/bash
# Elysium Web — build & serve
set -e

echo "⚡ Building Elysium Web (WASM + WebGL2)..."

# Build WASM
cargo build --target wasm32-unknown-unknown -p elysium-web --release

# Generate JS bindings
mkdir -p crates/elysium-web/pkg
wasm-bindgen --target web \
    --out-dir crates/elysium-web/pkg \
    target/wasm32-unknown-unknown/release/elysium_web.wasm

# Copy HTML
cp crates/elysium-web/index.html crates/elysium-web/pkg/

# Report size
WASM_SIZE=$(du -h crates/elysium-web/pkg/elysium_web_bg.wasm | cut -f1)
echo "✅ Build complete!"
echo "   WASM: $WASM_SIZE"
echo "   Output: crates/elysium-web/pkg/"
echo ""
echo "🌐 To serve locally:"
echo "   cd crates/elysium-web/pkg && python3 -m http.server 8080"
echo "   Then open: http://localhost:8080/index.html"
