pub mod fold;
pub mod layers;

pub use fold::{domain_point, fold_layer, fold_value};
pub use layers::{
    replay_commit_phase, verify_queries, ChallengeSource, FriCommitPhaseProof, FriProver, FriRoundOpening,
};

#[cfg(test)]
mod tests {
    use super::*;
    use field::{Field, ToyField};
    use merkle::Poseidon;
    use poly::lde::low_degree_extend;
        struct TestTranscript<'h, F: Field> {
        hasher: &'h Poseidon<F>,
        state: [F; merkle::poseidon::T],
    }

    impl<'h, F: Field> TestTranscript<'h, F> {
        fn new(hasher: &'h Poseidon<F>) -> Self {
            TestTranscript { hasher, state: [F::zero(); merkle::poseidon::T] }
        }
    }

    impl<'h, F: Field> ChallengeSource<F> for TestTranscript<'h, F> {
        fn absorb(&mut self, value: F) {
            self.state[0] += value;
            self.state = self.hasher.permute(self.state);
        }

        fn squeeze_field(&mut self) -> F {
            let out = self.state[0];
            self.state = self.hasher.permute(self.state);
            out
        }

        fn squeeze_index(&mut self, bound: usize) -> usize {
            (self.squeeze_field().to_canonical_u64() % bound as u64) as usize
        }
    }

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    fn run_fri_roundtrip(
        coeffs: Vec<ToyField>,
        blowup: usize,
        final_layer_size: usize,
        final_poly_num_coeffs: usize,
        num_queries: usize,
    ) -> bool {
        let hasher: Poseidon<ToyField> = Poseidon::new(5);
        let n = coeffs.len();
        let domain_size = n * blowup;
        let offset = tf(2);
        let evals = low_degree_extend(&coeffs, blowup, offset);

        let mut prover_transcript = TestTranscript::new(&hasher);
        let prover = FriProver::commit(
            &hasher,
            evals,
            offset,
            final_layer_size,
            final_poly_num_coeffs,
            &mut prover_transcript,
        );
        let bound = prover.index_bound();
        let indices: Vec<usize> = (0..num_queries).map(|_| prover_transcript.squeeze_index(bound)).collect();
        let openings = prover.open(&indices);
        let commit_proof = prover.proof.clone();

        let mut verifier_transcript = TestTranscript::new(&hasher);
        let betas = replay_commit_phase(&commit_proof, &mut verifier_transcript);
        let verifier_indices: Vec<usize> =
            (0..num_queries).map(|_| verifier_transcript.squeeze_index(domain_size / 2)).collect();
        assert_eq!(indices, verifier_indices, "prover/verifier index derivation diverged");

        verify_queries(
            &hasher,
            &commit_proof,
            &betas,
            offset,
            domain_size,
            final_layer_size,
            final_poly_num_coeffs,
            &verifier_indices,
            &openings,
        )
    }

    #[test]
    fn accepts_a_genuine_low_degree_polynomial() {
        let coeffs: Vec<ToyField> = vec![1, 2, 3, 4].into_iter().map(tf).collect();
        assert!(run_fri_roundtrip(coeffs, 4, 4, 1, 12));
    }

    #[test]
    fn accepts_a_constant_polynomial() {
        let coeffs = vec![tf(7)];
        assert!(run_fri_roundtrip(coeffs, 8, 2, 1, 8));
    }

    #[test]
    fn accepts_over_goldilocks_too() {
        use field::Goldilocks;
        let hasher: Poseidon<Goldilocks> = Poseidon::new(7);
        let coeffs: Vec<Goldilocks> = vec![1u64, 2, 3, 4].into_iter().map(Goldilocks::from_u64).collect();
        let blowup = 4;
        let domain_size = coeffs.len() * blowup;
        let offset = Goldilocks::from_u64(2);
        let evals = low_degree_extend(&coeffs, blowup, offset);

        let mut prover_transcript = TestTranscript::new(&hasher);
        let prover = FriProver::commit(&hasher, evals, offset, 4, 1, &mut prover_transcript);
        let bound = prover.index_bound();
        let indices: Vec<usize> = (0..10).map(|_| prover_transcript.squeeze_index(bound)).collect();
        let openings = prover.open(&indices);
        let proof = prover.proof.clone();

        let mut verifier_transcript = TestTranscript::new(&hasher);
        let betas = replay_commit_phase(&proof, &mut verifier_transcript);
        let verifier_indices: Vec<usize> =
            (0..10).map(|_| verifier_transcript.squeeze_index(domain_size / 2)).collect();

        assert!(verify_queries(
            &hasher,
            &proof,
            &betas,
            offset,
            domain_size,
            4,
            1,
            &verifier_indices,
            &openings
        ));
    }

    #[test]
    fn rejects_data_with_no_low_degree_structure() {
        let hasher: Poseidon<ToyField> = Poseidon::new(5);
        let domain_size = 16usize;
        let final_layer_size = 4;
        let final_poly_num_coeffs = 1;
        let offset = tf(2);
        let evals: Vec<ToyField> = (0..domain_size as u64).map(tf).collect();

        let mut prover_transcript = TestTranscript::new(&hasher);
        let prover = FriProver::commit(
            &hasher,
            evals,
            offset,
            final_layer_size,
            final_poly_num_coeffs,
            &mut prover_transcript,
        );
        let bound = prover.index_bound();
        let num_queries = 20;
        let indices: Vec<usize> = (0..num_queries).map(|_| prover_transcript.squeeze_index(bound)).collect();
        let openings = prover.open(&indices);
        let proof = prover.proof.clone();

        let mut verifier_transcript = TestTranscript::new(&hasher);
        let betas = replay_commit_phase(&proof, &mut verifier_transcript);
        let verifier_indices: Vec<usize> =
            (0..num_queries).map(|_| verifier_transcript.squeeze_index(domain_size / 2)).collect();

        let accepted = verify_queries(
            &hasher,
            &proof,
            &betas,
            offset,
            domain_size,
            final_layer_size,
            final_poly_num_coeffs,
            &verifier_indices,
            &openings,
        );
        assert!(!accepted);
    }

    #[test]
    fn rejects_tampered_final_polynomial() {
        let hasher: Poseidon<ToyField> = Poseidon::new(5);
        let coeffs: Vec<ToyField> = vec![1, 2, 3, 4].into_iter().map(tf).collect();
        let domain_size = coeffs.len() * 4;
        let offset = tf(2);
        let evals = low_degree_extend(&coeffs, 4, offset);

        let mut prover_transcript = TestTranscript::new(&hasher);
        let prover = FriProver::commit(&hasher, evals, offset, 4, 1, &mut prover_transcript);
        let bound = prover.index_bound();
        let indices: Vec<usize> = (0..10).map(|_| prover_transcript.squeeze_index(bound)).collect();
        let openings = prover.open(&indices);
        let mut tampered = prover.proof.clone();
        tampered.final_poly_coeffs[0] += tf(1);

        let mut verifier_transcript = TestTranscript::new(&hasher);
        let betas = replay_commit_phase(&tampered, &mut verifier_transcript);
        assert!(!verify_queries(&hasher, &tampered, &betas, offset, domain_size, 4, 1, &indices, &openings));
    }

    #[test]
    fn rejects_final_polynomial_with_wrong_coefficient_count() {
        let hasher: Poseidon<ToyField> = Poseidon::new(5);
        let coeffs: Vec<ToyField> = vec![1, 2, 3, 4].into_iter().map(tf).collect();
        let domain_size = coeffs.len() * 4;
        let offset = tf(2);
        let evals = low_degree_extend(&coeffs, 4, offset);

        let mut prover_transcript = TestTranscript::new(&hasher);
        let prover = FriProver::commit(&hasher, evals, offset, 4, 1, &mut prover_transcript);
        let bound = prover.index_bound();
        let indices: Vec<usize> = (0..10).map(|_| prover_transcript.squeeze_index(bound)).collect();
        let openings = prover.open(&indices);

        let mut oversized = prover.proof.clone();
        oversized.final_poly_coeffs.resize(4, ToyField::from_u64(0));

        let mut verifier_transcript = TestTranscript::new(&hasher);
        let betas = replay_commit_phase(&oversized, &mut verifier_transcript);

        assert!(!verify_queries(&hasher, &oversized, &betas, offset, domain_size, 4, 1, &indices, &openings));
    }
}
