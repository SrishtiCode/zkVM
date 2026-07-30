# zkvm
**The easiest zkVM to understand.**

This is a STARK-based zkVM built from scratch, with every stage of the proving
pipeline made visible and interactive:

- **CPU simulator** — step through a toy program instruction by instruction
- **Execution trace viewer** — the table of register/memory state per cycle
- **AIR visualizer** — the constraint polynomials that make a trace "valid"
- **Polynomial viewer** — interpolation, low-degree extension, degree bounds
- **FRI step visualizer** — the folding protocol that proves low degree
- **Proof explorer** — Merkle paths, query openings, verifier checks

There are production zkVMs with far more features (RISC Zero, SP1, Cairo VM).
This project optimizes for a different thing: opening the hood so the whole
mechanism is legible, end to end, in one sitting.

## Why STARKs

STARKs need no trusted setup — the whole system is built from two primitives:
**hash functions** and **polynomial arithmetic over a finite field**. That
makes them unusually teachable compared to SNARK constructions that route
through pairing-based cryptography.

## Pipeline

```
program
  │
  ▼
CPU simulator ──▶ execution trace ──▶ AIR constraints
                                            │
                                            ▼
                polynomials ──▶ composition poly ──▶ FRI folding
                                                            │
                                                            ▼
                                  proof object ──▶ verifier
```

Every stage writes a JSON artifact (`trace.json`, `air.json`, `lde.json`,
`fri_rounds.json`, `proof.json`) consumed by a matching visualizer in `web/`.
This keeps "the math" and "the visualization" cleanly decoupled — you can
read or test any stage in isolation.

## Status

🚧 Early scaffolding. Building in order:

- [ ] Phase 0 — field arithmetic + polynomial ops (`crates/field`, `crates/poly`)
- [ ] Phase 1 — toy STARK for a trivial statement (no CPU yet), validating the
      full pipeline: interpolate → LDE → constraints → FRI → verify
- [ ] Phase 2 — minimal ISA + CPU simulator (`crates/isa`)
- [ ] Phase 3 — AIR for that ISA (`crates/air`)
- [ ] Phase 4 — Merkle commitments (`crates/merkle`)
- [ ] Phase 5 — FRI protocol (`crates/fri`)
- [ ] Phase 6 — Fiat–Shamir transcript, non-interactive proofs (`crates/transcript`)
- [ ] Phase 7 — prover + verifier assembly (`crates/prover`, `crates/verifier`)
- [ ] Phase 8 — the six web visualizers (`web/`)
- [ ] Phase 9 — WASM build so visualizers run a *live* prover in-browser

See `docs/` for a write-up per stage as each one lands.

## Repo layout

```
crates/         one crate per pipeline stage (field, poly, merkle, isa, air,
                 fri, transcript, prover, verifier, viz-export)
bin/zkvm-cli/   command-line entry point: run / prove / verify
examples/       toy programs (fibonacci, squares, collatz)
docs/           one explainer per pipeline stage
web/            the six interactive visualizers (React/TypeScript)
wasm/           compiles the real prover to WASM for live in-browser proving
tests/          integration + conformance tests
```

## Running it

```bash
# core pipeline (once phase 1+ lands)
cargo run -p zkvm-cli -- run examples/fibonacci/program.zkasm
cargo run -p zkvm-cli -- prove examples/fibonacci/program.zkasm
cargo run -p zkvm-cli -- verify examples/fibonacci/proof.json

# visualizers (once phase 8 lands)
cd web && npm install && npm run dev
```

## Design choices worth knowing about

- **Toy field mode.** Real STARK fields (e.g. Goldilocks, `2^64 - 2^32 + 1`)
  are too large to display meaningfully. A small prime field (mod 97) with a
  tiny domain (8–16 elements) is used by default so every number in the
  polynomial/FRI viewers is human-graspable. A "real mode" toggle switches to
  the production-size field to show the same code scales.
- **Toy ISA, not RISC-V.** A handful of opcodes (add, mul, jump, load, store,
  halt) — enough to be interesting, not so much that AIR constraints for
  memory consistency and byte range-checks drown out the core ideas.
- **JSON artifacts as the seam.** The Rust core and the web frontend never
  share code directly; they share a JSON schema. This makes every stage
  independently testable and keeps the "explain it" layer from leaking into
  the "prove it" layer.

## References

This project stands on the shoulders of excellent existing work:

- [STARK 101](https://starkware.co) (StarkWare) — the tutorial this project's
  Phase 1 "toy STARK" is modeled after
- [Winterfell](https://github.com/facebook/winterfell) — a well-documented
  Rust STARK library, useful as a reference for AIR/FRI structuring
- RISC Zero, SP1, Cairo VM — production zkVMs worth studying once the toy
  pipeline works end to end

## License

MIT
