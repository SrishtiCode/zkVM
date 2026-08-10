// Types mirroring viz-export's serde structs exactly. Kept as plain
// interfaces (no runtime validation library) — this is a from-scratch
// educational project, and the export side is the only producer of this
// JSON, so a schema-validation layer would be defending against a
// mismatch that a TypeScript compile error in this file already catches
// whenever the Rust side's shape changes.

export interface InstructionJson {
  mnemonic: string;
  text: string;
}

export interface TraceRowJson {
  cycle: number;
  pc: number;
  registers: number[];
  instruction_index: number;
  mem_read: [number, number] | null;
  mem_write: [number, number] | null;
}

export interface AirRowJson {
  cycle: number;
  pc: number;
  registers: number[];
  sel: number[];
  opcode_name: string;
  is_write_r: number[];
  is_read_a_r: number[];
  is_read_b_r: number[];
  addr: number;
  imm: number;
  mem_read_value: number;
  mem_write_value: number;
  jnz_is_zero: number;
  jnz_inv: number;
  transition_checks: [string, number][];
}

export interface CpuExport {
  field_modulus: number;
  program: InstructionJson[];
  trace: TraceRowJson[];
  air_rows: AirRowJson[];
  final_registers: number[];
  final_memory: number[];
}

export interface PolynomialExport {
  field_modulus: number;
  trace_len: number;
  trace_values: number[];
  coefficients: number[];
  blowup_factor: number;
  domain_offset: number;
  lde_domain: number[];
  lde_evaluations: number[];
}

export interface FriRoundExport {
  round: number;
  domain_size: number;
  domain: number[];
  evaluations: number[];
  merkle_root: number | null;
  beta: number | null;
}

export interface FriExport {
  rounds: FriRoundExport[];
  final_poly_coeffs: number[];
}

export interface MerklePathJson {
  leaf_index: number;
  siblings: number[];
}

export interface QueryJson {
  index: number;
  trace_value: number;
  trace_path: MerklePathJson;
}

export interface ProofExport {
  statement: string;
  start: number;
  step: number;
  end: number;
  trace_root: number;
  num_queries: number;
  queries: QueryJson[];
  fri_layer_roots: number[];
  fri_final_poly_coeffs: number[];
  accepted: boolean;
}

const ARTIFACTS_BASE = `${import.meta.env.BASE_URL}artifacts`;

async function fetchArtifact<T>(filename: string): Promise<T> {
  const res = await fetch(`${ARTIFACTS_BASE}/${filename}`);
  if (!res.ok) {
    throw new Error(
      `failed to fetch ${filename} (${res.status}). Did you run ` +
        `'cargo run -p viz-export --bin export' from the repo root first?`,
    );
  }
  return res.json() as Promise<T>;
}

export const fetchTrace = () => fetchArtifact<CpuExport>('trace.json');
export const fetchAir = () => fetchArtifact<AirRowJson[]>('air.json');
export const fetchPolynomial = () => fetchArtifact<PolynomialExport>('lde.json');
export const fetchFri = () => fetchArtifact<FriExport>('fri_rounds.json');
export const fetchProof = () => fetchArtifact<ProofExport>('proof.json');
