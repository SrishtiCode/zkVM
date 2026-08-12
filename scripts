set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v rust-lld >/dev/null 2>&1; then
  echo "error: rust-lld not found on PATH." >&2
  echo "  Run once: ln -sf \$(command -v lld-17) /usr/local/bin/rust-lld" >&2
  echo "  (install with: apt-get install -y lld-17)" >&2
  exit 1
fi

echo "Building crates/wasm for wasm32-unknown-unknown..."
RUSTC_BOOTSTRAP=1 cargo build \
  --package wasm \
  --target wasm32-unknown-unknown \
  -Z build-std=core,alloc \
  --release

mkdir -p web/public/wasm
cp target/wasm32-unknown-unknown/release/wasm.wasm web/public/wasm/zkvm.wasm
echo "wrote web/public/wasm/zkvm.wasm ($(wc -c < web/public/wasm/zkvm.wasm) bytes)"
