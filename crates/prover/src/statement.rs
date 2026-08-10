use air::NUM_COLUMNS;
use field::Field;
use isa::NUM_REGISTERS;
use merkle::MerklePath;

#[derive(Debug, Clone, Copy)]
pub struct ProverParams {
    pub blowup_factor: usize,
    pub fri_final_poly_num_coeffs: usize,
    pub num_queries: usize,
}

pub const COMPOSITION_DEGREE_BOUND_FACTOR: usize = 8;

impl ProverParams {
    pub fn fri_final_layer_size(&self) -> usize {
        assert!(
            self.blowup_factor % COMPOSITION_DEGREE_BOUND_FACTOR == 0
                && self.blowup_factor > COMPOSITION_DEGREE_BOUND_FACTOR,
            "blowup_factor must be a multiple of {COMPOSITION_DEGREE_BOUND_FACTOR} and greater than \
             it, so FRI's final layer has real redundancy beyond just covering this AIR's degree \
             bound (got blowup_factor={})",
            self.blowup_factor
        );
        self.fri_final_poly_num_coeffs * (self.blowup_factor / COMPOSITION_DEGREE_BOUND_FACTOR)
    }
}

impl Default for ProverParams {
    fn default() -> Self {
        ProverParams { blowup_factor: 32, fri_final_poly_num_coeffs: 1, num_queries: 32 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicClaim<F: Field> {
    pub claimed_final_registers: [F; NUM_REGISTERS],
}

#[derive(Debug, Clone)]
pub struct TraceRowOpening<F: Field> {
    pub values: Vec<F>,
    pub path: MerklePath<F>,
}

#[derive(Debug, Clone)]
pub struct QueryOpening<F: Field> {
    pub trace_here: TraceRowOpening<F>,
    pub trace_here_next: TraceRowOpening<F>,
    pub trace_paired: TraceRowOpening<F>,
    pub trace_paired_next: TraceRowOpening<F>,
    pub fri: Vec<fri::FriRoundOpening<F>>,
}

#[derive(Debug, Clone)]
pub struct Proof<F: Field> {
    pub claim: PublicClaim<F>,
    pub params: ProverParams,
    pub trace_len: usize,
    pub trace_root: F,
    pub program_hash: F,
    pub fri_commit: fri::FriCommitPhaseProof<F>,
    pub queries: Vec<QueryOpening<F>>,
}

pub fn assert_row_shape<F: Field>(values: &[F]) {
    assert_eq!(values.len(), NUM_COLUMNS, "trace row opening must carry exactly NUM_COLUMNS values");
}
