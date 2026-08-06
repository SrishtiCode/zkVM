pub mod air;
pub mod fri;
pub mod hash;
pub mod prover;
pub mod transcript;
pub mod verifier;

pub use air::PublicInputs;
pub use prover::{Proof, ProofParams};

#[cfg(test)]
mod tests {
    use super::*;
    use field::{Field, Goldilocks, ToyField};

    #[test]
    fn honest_proof_verifies_toyfield() {
        let public = PublicInputs { trace_len: 8, start: ToyField::from_u64(3), step: ToyField::from_u64(2) };
        let proof = prover::prove(public, ProofParams::default());
        assert!(verifier::verify(&proof));
    }

    #[test]
    fn honest_proof_verifies_goldilocks() {
        let public =
            PublicInputs { trace_len: 16, start: Goldilocks::from_u64(100), step: Goldilocks::from_u64(7) };
        let proof = prover::prove(public, ProofParams::default());
        assert!(verifier::verify(&proof));
    }

    #[test]
    fn honest_proof_verifies_across_trace_lengths_and_params() {
        for &trace_len in &[2usize, 4, 8, 32] {
            for &blowup in &[2usize, 4, 8] {
                let public = PublicInputs {
                    trace_len,
                    start: Goldilocks::from_u64(1),
                    step: Goldilocks::from_u64(1),
                };
                let params = ProofParams {
                    blowup_factor: blowup,
                    fri_final_poly_num_coeffs: 1,
                    num_queries: 8,
                };
                let proof = prover::prove(public, params);
                assert!(verifier::verify(&proof), "trace_len={trace_len} blowup={blowup}");
            }
        }
    }

    #[test]
    fn tampered_trace_opening_is_rejected() {
        let public = PublicInputs { trace_len: 8, start: ToyField::from_u64(3), step: ToyField::from_u64(2) };
        let mut proof = prover::prove(public, ProofParams::default());
        proof.queries[0].trace_a.value_here += ToyField::from_u64(1);
        assert!(!verifier::verify(&proof));
    }

    #[test]
    fn tampered_public_end_value_is_rejected() {
        let public = PublicInputs { trace_len: 8, start: ToyField::from_u64(3), step: ToyField::from_u64(2) };
        let proof = prover::prove(public, ProofParams::default());
        let mut tampered = proof;
        tampered.public.step = ToyField::from_u64(3);
        assert!(!verifier::verify(&tampered));
    }

    #[test]
    fn proof_for_wrong_start_value_is_rejected() {
        let honest_public =
            PublicInputs { trace_len: 8, start: ToyField::from_u64(3), step: ToyField::from_u64(2) };
        let proof = prover::prove(honest_public, ProofParams::default());
        let mut tampered = proof;
        tampered.public.start = ToyField::from_u64(4);
        assert!(!verifier::verify(&tampered));
    }

    #[test]
    fn dishonest_witness_is_rejected() {
        let public = PublicInputs { trace_len: 8, start: ToyField::from_u64(3), step: ToyField::from_u64(2) };
        let mut broken_trace = air::generate_trace(&public);
        broken_trace[5] = broken_trace[5] + ToyField::from_u64(1);
        let proof = prover::prove_with_trace(public, ProofParams::default(), broken_trace);
        assert!(!verifier::verify(&proof));
    }

    #[test]
    fn corrupted_fri_final_polynomial_is_rejected() {
        let public = PublicInputs { trace_len: 8, start: ToyField::from_u64(3), step: ToyField::from_u64(2) };
        let mut proof = prover::prove(public, ProofParams::default());
        proof.fri_commit.final_poly_coeffs[0] += ToyField::from_u64(1);
        assert!(!verifier::verify(&proof));
    }

    #[test]
    fn corrupted_fri_layer_root_is_rejected() {
        let public = PublicInputs { trace_len: 8, start: ToyField::from_u64(3), step: ToyField::from_u64(2) };
        let mut proof = prover::prove(public, ProofParams::default());
        proof.fri_commit.layer_roots[0] ^= 1;
        assert!(!verifier::verify(&proof));
    }
}
