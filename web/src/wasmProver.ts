// A thin wrapper around crates/wasm's exports. There's no wasm-bindgen
// glue here — that crate is deliberately no_std (see its lib.rs for why),
// so the boundary is plain `extern "C"` functions over integers: call
// `provePower` once, then read results back through the getters.

export interface PowerProofResult {
  accepted: boolean;
  resultValue: bigint;
  traceLen: number;
  numQueries: number;
  traceRootLow: bigint;
}

interface WasmExports {
  prove_power: (base: bigint, exponent: bigint) => number;
  last_accepted: () => number;
  last_result_value: () => bigint;
  last_trace_len: () => number;
  last_num_queries: () => number;
  last_trace_root_low: () => bigint;
}

let cachedInstance: WasmExports | null = null;

async function loadModule(): Promise<WasmExports> {
  if (cachedInstance) return cachedInstance;
  const url = `${import.meta.env.BASE_URL}wasm/zkvm.wasm`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`failed to fetch ${url} (${res.status}). Did you run scripts/build-wasm.sh?`);
  }
  const bytes = await res.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  cachedInstance = instance.exports as unknown as WasmExports;
  return cachedInstance;
}

/** Runs the real prover, then the real verifier, entirely inside the
 * browser's WASM sandbox — no server round trip. Typically takes several
 * seconds: this is a genuine STARK (interpolation, LDE, Merkle commits,
 * FRI folding, Fiat-Shamir) over a 128-cycle trace, not a mocked delay. */
export async function provePower(base: bigint, exponent: bigint): Promise<PowerProofResult> {
  const wasm = await loadModule();
  const accepted = wasm.prove_power(base, exponent) === 1;
  return {
    accepted,
    resultValue: wasm.last_result_value(),
    traceLen: wasm.last_trace_len(),
    numQueries: wasm.last_num_queries(),
    traceRootLow: wasm.last_trace_root_low(),
  };
}

export const MAX_EXPONENT = 25;
