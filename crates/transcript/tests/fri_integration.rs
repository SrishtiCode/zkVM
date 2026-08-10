use field::{Field, ToyField};
use fri::{replay_commit_phase, verify_queries, FriProver};
use merkle::Poseidon;
use poly::lde::low_degree_extend;
use transcript::Transcript;

fn tf(x: u64) -> ToyField{
    ToyField::from_u64(x)
}

fn prove_and_verify(
    coeffs: Vec<ToyField>,
    blowup: usize,
    final_layer_size: usize,
    final_poly_num_coeffs: usize,
    num_queries: usize, 
) -> bool{
    let hasher: Poseidon<ToyField> = Poseidon::new(5);
    let domain_size = coeffs.len() * blowup;
    let offset = tf(3);
    let evals = low_degree_extend(&coeffs, blowup, offset);

    let mut prover_transcript = Transcript::new(&hasher, "fri-integratio-test-v1");
    prover_transcript.absorb_u64(domain_size as u64);
    let prover = FriProver::commit(
        &hasher,
        evals,
        offset,
        final_layer_size,
        final_poly_num_coeffs,
        &mut prover_transcript,
    );
    let bound = prover.index_bound();
    let query_indices: Vec<usize> = (0..num_queries).map(|_| prover_transcript.squeeze_index(bound)).collect();
    let openings = prover.open(&query_indices);
    let proof = prover.proof.clone();
    
    let mut verifier_transcript = Transcript::new(&hasher, "fri-integration-test-v1");
    verifier_transcript.absorb_u64(domain_size as u64);
    let betas = replay_commit_phase(&proof, &mut verifier_transript);
    let verifier_indices: Vec<usize> = (0..num_queries).map(|_| verifier_transcipt.squeeze_index(domain_size/2)).collect();       

    assert_eq!(
        query_indices, verifier_indices,"the whole point of Fiat-Shamir: two independently-run transcripts over the same \ 
        data must derive identical challenges without any direct communication"
    );

    verify_queries(
        &hasher,
        &proof,
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
fn honest_fri_proof_is_accepted_end_to_end() {
    let coeffs: Vec<ToyField> = vec![1, 2, 3, 4].into_iter().map(tf).collect();
    assert!(prove_and_verify(coeffs, 4, 4, 1, 16));
}

#[test]
fn different_domain_separator_label_would_desync_prover_and_verifier() {
    let hasher: Poseidon<ToyField> = Poseidon::new(5);
    let mut a = Transcript::new(&hasher, "protocol-v1");
    let mut b = Transcript::new(&hasher, "protocol-v2");
    a.absorb(tf(42));
    b.absorb(tf(42));
    assert_ne!(a.squeeze_field(), b.squeeze_field());
}

#[test]
fn tampered_proof_is_rejected_end_to_end() {
    let hasher: Poseidon<ToyField> = Poseidon::new(5);
    let coeffs: Vec<ToyField> = vec![1, 2, 3, 4].into_iter().map(tf).collect();
    let domain_size = coeffs.len() * 4;
    let offset = tf(3);
    let evals = low_degree_extend(&coeffs, 4, offset);

    let mut prover_transcript = Transcript::new(&hasher, "fri-integration-test-v1");
    prover_transcript.absorb_u64(domain_size as u64);
    let prover = FriProver::commit(&hasher, evals, offset, 4, 1, &mut prover_transcript);
    let bound = prover.index_bound();
    let indices: Vec<usize> = (0..16).map(|_| prover_transcript.squeeze_index(bound)).collect();
    let openings = prover.open(&indices);

    let mut tampered_proof = prover.proof.clone();
    tampered_proof.final_poly_coeffs[0] += tf(1);

    let mut verifier_transcript = Transcript::new(&hasher, "fri-integration-test-v1");
    verifier_transcript.absorb_u64(domain_size as u64);
    let betas = replay_commit_phase(&tampered_proof, &mut verifier_transcript);
    assert!(!verify_queries(
        &hasher,
        &tampered_proof,
        &betas,
        offset,
        domain_size,
        4,
        1,
        &indices,
        &openings
    ));
}
