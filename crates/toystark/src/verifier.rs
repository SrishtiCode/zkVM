use crate::air;
use crate::fri::{self, domain_point};
use crate::hash::hash_leaf;
use crate::prover::{Proof, TracePointOpening};
use crate::transcript::Transcript;
use field::Field;

fn domain_separator<F: Field>(public: &air::PublicInputs<F>, params: &crate::prover::ProofParams) -> Vec<u64> {
    vec![
        public.trace_len as u64,
        public.start.to_canonical_u64(),
        public.step.to_canonical_u64(),
        params.blowup_factor as u64,
    ]
}

fn verify_trace_point<F: Field>(
    opening: &TracePointOpening<F>,
    trace_root: u64,
    index_here: usize,
    index_next: usize,
) -> bool {
    opening.path_here.leaf_index == index_here
        && opening.path_next.leaf_index == index_next
        && opening.path_here.verify(trace_root, hash_leaf(opening.value_here.to_canonical_u64()))
        && opening.path_next.verify(trace_root, hash_leaf(opening.value_next.to_canonical_u64()))
}

pub fn verify<F: Field>(proof: &Proof<F>) -> bool {
    let n = proof.public.trace_len;
    if !n.is_power_of_two() {
        return false;
    }
    let big_n = n * proof.params.blowup_factor;
    let half = big_n / 2;
    let offset = F::from_u64(3);
    let log_big_n = big_n.trailing_zeros();
    let big_generator = F::root_of_unity(log_big_n);

    let mut transcript = Transcript::new(&domain_separator(&proof.public, &proof.params));
    transcript.absorb(proof.trace_root);
    let alphas =
        (transcript.squeeze_field::<F>(), transcript.squeeze_field::<F>(), transcript.squeeze_field::<F>());

    let betas = fri::replay_commit_phase(&proof.fri_commit, &mut transcript);

    let bound = half;
    if bound == 0 || proof.params.num_queries != proof.queries.len() {
        return false;
    }
    let indices: Vec<usize> =
        (0..proof.params.num_queries).map(|_| transcript.squeeze_index(bound)).collect();

    for (&idx0, query) in indices.iter().zip(&proof.queries) {
        let idx1 = idx0 + half;
        let next0 = (idx0 + proof.params.blowup_factor) % big_n;
        let next1 = (idx1 + proof.params.blowup_factor) % big_n;

        if !verify_trace_point(&query.trace_a, proof.trace_root, idx0, next0) {
            return false;
        }
        if !verify_trace_point(&query.trace_b, proof.trace_root, idx1, next1) {
            return false;
        }

        if query.fri.is_empty() {
            return false;
        }
        let round0 = &query.fri[0];

        let x_a = domain_point(offset, big_generator, idx0, 0);
        let x_b = domain_point(offset, big_generator, idx1, 0);
        let composition_a = air::evaluate_composition_at_query_point(
            x_a,
            query.trace_a.value_here,
            query.trace_a.value_next,
            &proof.public,
            alphas,
        );
        let composition_b = air::evaluate_composition_at_query_point(
            x_b,
            query.trace_b.value_here,
            query.trace_b.value_next,
            &proof.public,
            alphas,
        );

        if composition_a != round0.value_a || composition_b != round0.value_b {
            return false;
        }
    }

    let fri_openings: Vec<Vec<_>> = proof.queries.iter().map(|q| q.fri.clone()).collect();
    fri::verify_queries(
        &proof.fri_commit,
        &betas,
        offset,
        big_n,
        proof.params.fri_final_layer_size(),
        proof.params.fri_final_poly_num_coeffs,
        &indices,
        &fri_openings,
    )
}
