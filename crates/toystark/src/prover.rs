use crate::air::{self, PublicInputs};
use crate::fri::{FriCommitPhaseProof, FriProver, FriRoundOpening};
use crate::hash::{hash_leaf, MerklePath, MerkleTree};
use crate::transcript::Transcript;
use field::Field;
use poly::interpolate::fft_interpolate;
use poly::lde::{coset_domain, low_degree_extend};

#[derive(Debug, Clone, Copy)]
pub struct ProofParams {
    pub blowup_factor: usize,
    pub fri_final_poly_num_coeffs: usize,
    pub num_queries: usize,
}

impl ProofParams {
    pub(crate) fn fri_final_layer_size(&self) -> usize {
        self.blowup_factor * self.fri_final_poly_num_coeffs
    }
}

impl Default for ProofParams {
    fn default() -> Self {
        ProofParams { blowup_factor: 4, fri_final_poly_num_coeffs: 1, num_queries: 24 }
    }
}

#[derive(Debug, Clone)]
pub struct TracePointOpening<F: Field> {
    pub value_here: F,
    pub path_here: MerklePath,
    pub value_next: F,
    pub path_next: MerklePath,
}

#[derive(Debug, Clone)]
pub struct QueryOpening<F: Field> {
    pub trace_a: TracePointOpening<F>,
    pub trace_b: TracePointOpening<F>,
    pub fri: Vec<FriRoundOpening<F>>,
}

#[derive(Debug, Clone)]
pub struct Proof<F: Field> {
    pub public: PublicInputs<F>,
    pub params: ProofParams,
    pub trace_root: u64,
    pub fri_commit: FriCommitPhaseProof<F>,
    pub query_indices: Vec<usize>,
    pub queries: Vec<QueryOpening<F>>,
}

fn domain_separator<F: Field>(public: &PublicInputs<F>, params: &ProofParams) -> Vec<u64> {
    vec![
        public.trace_len as u64,
        public.start.to_canonical_u64(),
        public.step.to_canonical_u64(),
        params.blowup_factor as u64,
    ]
}

pub fn prove_with_trace<F: Field>(
    public: PublicInputs<F>,
    params: ProofParams,
    trace: Vec<F>,
) -> Proof<F> {
    assert_eq!(trace.len(), public.trace_len);
    let n = public.trace_len;
    assert!(n.is_power_of_two(), "trace length must be a power of two");
    let big_n = n * params.blowup_factor;

    let trace_coeffs = fft_interpolate(&trace).coeffs().to_vec();
    let offset = F::from_u64(3);
    let trace_lde_evals = low_degree_extend(&trace_coeffs, params.blowup_factor, offset);
    let trace_lde_domain = coset_domain(big_n, offset);

    let trace_tree = MerkleTree::build(
        &trace_lde_evals.iter().map(|v| hash_leaf(v.to_canonical_u64())).collect::<Vec<_>>(),
    );
    let trace_root = trace_tree.root();

    let mut transcript = Transcript::new(&domain_separator(&public, &params));
    transcript.absorb(trace_root);
    let alphas = (
        transcript.squeeze_field::<F>(),
        transcript.squeeze_field::<F>(),
        transcript.squeeze_field::<F>(),
    );

    let composition_evals = air::evaluate_composition(
        &trace_lde_evals,
        &trace_lde_domain,
        params.blowup_factor,
        &public,
        alphas,
    );

    let fri_prover = FriProver::commit(
        composition_evals,
        offset,
        params.fri_final_layer_size(),
        params.fri_final_poly_num_coeffs,
        &mut transcript,
    );

    let bound = fri_prover.index_bound();
    let query_indices: Vec<usize> =
        (0..params.num_queries).map(|_| transcript.squeeze_index(bound)).collect();
    let fri_openings = fri_prover.open(&query_indices);

    let half = big_n / 2;
    let trace_point_opening = |j: usize| -> TracePointOpening<F> {
        let next = (j + params.blowup_factor) % big_n;
        TracePointOpening {
            value_here: trace_lde_evals[j],
            path_here: trace_tree.open(j),
            value_next: trace_lde_evals[next],
            path_next: trace_tree.open(next),
        }
    };

    let queries: Vec<QueryOpening<F>> = query_indices
        .iter()
        .zip(fri_openings.into_iter())
        .map(|(&idx0, fri)| QueryOpening {
            trace_a: trace_point_opening(idx0),
            trace_b: trace_point_opening(idx0 + half),
            fri,
        })
        .collect();

    Proof { public, params, trace_root, fri_commit: fri_prover.proof.clone(), query_indices, queries }
}

pub fn prove<F: Field>(public: PublicInputs<F>, params: ProofParams) -> Proof<F> {
    let trace = air::generate_trace(&public);
    prove_with_trace(public, params, trace)
}
