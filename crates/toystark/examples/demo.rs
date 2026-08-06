use field::{Field, ToyField};
use toystark::{prover, verifier, PublicInputs};
use toystark::prover::ProofParams;

fn main() {
    println!("=== Phase 1 toy STARK demo ===\n");

    let public = PublicInputs { trace_len: 8, start: ToyField::from_u64(3), step: ToyField::from_u64(2) };
    let params = ProofParams::default();

    println!("Statement: trace of length {} starting at {}, step {} (public end = {})",
        public.trace_len, public.start.to_canonical_u64(), public.step.to_canonical_u64(), public.end().to_canonical_u64());
    println!(
        "Params: blowup_factor={}, fri_final_poly_num_coeffs={}, num_queries={}\n",
        params.blowup_factor, params.fri_final_poly_num_coeffs, params.num_queries
    );

    let trace = toystark::air::generate_trace(&public);
    println!("Witness trace: {:?}", trace.iter().map(|v| v.to_canonical_u64()).collect::<Vec<_>>());

    let proof = prover::prove(public, params);
    println!("\nCommitted trace Merkle root: 0x{:016x}", proof.trace_root);
    println!("FRI layer roots: {} layers", proof.fri_commit.layer_roots.len());
    for (i, root) in proof.fri_commit.layer_roots.iter().enumerate() {
        println!("  layer {i}: 0x{root:016x}");
    }
    println!(
        "FRI final polynomial coefficients: {:?}",
        proof.fri_commit.final_poly_coeffs.iter().map(|c| c.to_canonical_u64()).collect::<Vec<_>>()
    );
    println!("Number of queries: {}", proof.queries.len());

    let accepted = verifier::verify(&proof);
    println!("\nHonest proof verifies: {accepted}");
    assert!(accepted);

    let mut forged = prover::prove(public, params);
    forged.queries[0].trace_a.value_here += ToyField::from_u64(1);
    let forged_accepted = verifier::verify(&forged);
    println!("Tampered proof verifies: {forged_accepted} (expected false)");
    assert!(!forged_accepted);

    println!("\nAll checks passed.");
}
