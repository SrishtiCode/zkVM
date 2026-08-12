# zkvm web visualizers (Phases 8 & 9)

Seven views into the same pipeline built in Phases 0–7: CPU simulator,
execution trace viewer, AIR visualizer, polynomial viewer, FRI visualizer,
proof explorer, and — Phase 9 — a live prover running the real Rust code
compiled to WebAssembly.

## Data flow

There is no mocked data anywhere, including the live tab.

**Static viewers (Phase 8).** `crates/viz-export` runs the real Rust
pipeline (`isa` + `air` for the CPU-related viewers; `poly` + `merkle` +
`fri` + `transcript` for the polynomial/FRI/proof viewers, proving and
verifying a genuine arithmetic-progression statement) and writes five
JSON files to `web/public/artifacts/`. The React app only ever reads
those files.

**Live prover (Phase 9).** `crates/wasm` compiles the real `prover` +
`verifier` (Phases 3 and 7) to `wasm32-unknown-unknown`. The "Live prover"
tab calls straight into that WASM module — a fresh proof of
`base^exponent`, computed and verified in your browser, on every click.

```
# Static artifacts:
cargo run -p viz-export --bin export     # from the repo root -> web/public/artifacts/*.json

# Live prover WASM build (see scripts/build-wasm.sh for the one-time
# toolchain setup this needs, and why):
./scripts/build-wasm.sh                  # from the repo root -> web/public/wasm/zkvm.wasm

cd web
npm install
npm run dev                              # or: npm run build && npm run preview
```

Re-run the relevant build step any time the underlying Rust crates change.

## Layout

- `src/data/artifacts.ts` — TypeScript types mirroring `viz-export`'s
  `serde` structs exactly, plus `fetch()` helpers, for the six static
  viewers.
- `src/wasmProver.ts` — thin wrapper around `crates/wasm`'s hand-rolled
  `extern "C"` exports (no `wasm-bindgen` — that crate is `no_std`, see
  its docs for why), for the live prover tab.
- `src/useArtifact.tsx` — shared loading/error-state hook.
- `src/viewers/*.tsx` — the seven viewers, one file each.
- `src/App.tsx` — tab navigation across them.

## On the WASM build specifically

Getting `wasm32-unknown-unknown` working *without* `rustup` (no network
access to `static.rust-lang.org` in the environment this was built in)
took real toolchain archaeology — `rust-src` + `-Z build-std=core,alloc`
+ a version-matched `lld` linker, and porting the entire workspace to
`no_std` along the way (documented in `crates/field`'s and `crates/wasm`'s
doc comments, and in `scripts/build-wasm.sh`). If you're on a machine with
normal `rustup` access, `rustup target add wasm32-unknown-unknown` plus a
standard `cargo build --target wasm32-unknown-unknown --release` would
also work — the `no_std` port isn't a requirement of WASM in general,
just of *this* environment's constraints, and it's a proper part of the
workspace now either way.

