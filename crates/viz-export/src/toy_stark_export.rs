use field::Field;
use field::ToyField;
use fri::FriProver;
use merkle::{MerklePath, MerkleTree, Poseidon};
use poly::interpolate::fft_interpolate;
use poly::lde::{coset_domain, low_degree_extend};
use serde::Serialize;
use transcript::Transcript;

const TRACE_LEN: usize = 8;
const BLOWUP: usize = 4;
const START: u64 = 3;
const STEP: u64 = 2;

#[derive(Serialize)]
pub struct PolynomialExport {
    pub field_modulus: u64,
    pub trace_len: usize,
    pub trace_values: Vec<u64>,
    pub coefficients: Vec<u64>,
    pub blowup_factor: usize,
    pub domain_offset: u64,
    pub lde_domain: Vec<u64>,
    pub lde_evaluations: Vec<u64>,
}

#[derive(Serialize)]
pub struct FriRoundExport {
    pub round: usize,
    pub domain_size: usize,
    pub domain: Vec<u64>,
    pub evaluations: Vec<u64>,
    pub merkle_root: Option<u64>,
    pub beta: Option<u64>,
}

#[derive(Serialize)]
pub struct FriExport {
    pub rounds: Vec<FriRoundExport>,
    pub final_poly_coeffs: Vec<u64>,
}

#[derive(Serialize)]
pub struct MerklePathJson {
    pub leaf_index: usize,
    pub siblings: Vec<u64>,
}

#[derive(Serialize)]
pub struct QueryJson {
    pub index: usize,
    pub trace_value: u64,
    pub trace_path: MerklePathJson,
}

#[derive(Serialize)]
pub struct ProofExport {
    pub statement: String,
    pub start: u64,
    pub step: u64,
    pub end: u64,
    pub trace_root: u64,
    pub num_queries: usize,
    pub queries: Vec<QueryJson>,
    pub fri_layer_roots: Vec<u64>,
    pub fri_final_poly_coeffs: Vec<u64>,
    pub accepted: bool,
}

fn to_u64s(vals: &[ToyField]) -> Vec<u64> {
    vals.iter().map(|v| v.to_canonical_u64()).collect()
}

fn hasher() -> Poseidon<ToyField> {
    Poseidon::new(5)
}

pub fn export_polynomial() -> PolynomialExport {
    let trace: Vec<ToyField> = (0..TRACE_LEN as u64).map(|i| ToyField::from_u64(START + STEP * i)).collect();
    let coeffs = fft_interpolate(&trace).coeffs().to_vec();
    let mut padded = coeffs.clone();
    padded.resize(TRACE_LEN, ToyField::zero());
    let offset = ToyField::from_u64(2);
    let lde_evals = low_degree_extend(&padded, BLOWUP, offset);
    let lde_domain = coset_domain(TRACE_LEN * BLOWUP, offset);

    PolynomialExport {
        field_modulus: ToyField::modulus(),
        trace_len: TRACE_LEN,
        trace_values: to_u64s(&trace),
        coefficients: to_u64s(&coeffs),
        blowup_factor: BLOWUP,
        domain_offset: offset.to_canonical_u64(),
        lde_domain: to_u64s(&lde_domain),
        lde_evaluations: to_u64s(&lde_evals),
    }
}

fn export_fri_rounds(offset: ToyField) -> FriExport {
    let trace: Vec<ToyField> = (0..TRACE_LEN as u64).map(|i| ToyField::from_u64(START + STEP * i)).collect();
    let mut coeffs = fft_interpolate(&trace).coeffs().to_vec();
    coeffs.resize(TRACE_LEN, ToyField::zero());
    let mut evals = low_degree_extend(&coeffs, BLOWUP, offset);
    let mut domain = coset_domain(TRACE_LEN * BLOWUP, offset);

    let h = hasher();
    let mut rounds = Vec::new();
    let final_layer_size = 2;
    let mut round_idx = 0;
    let mut beta_seed = ToyField::from_u64(11); 
    loop {
        let root = MerkleTree::from_values(&h, &evals).root();
        let beta = if evals.len() > final_layer_size { Some(beta_seed) } else { None };
        rounds.push(FriRoundExport {
            round: round_idx,
            domain_size: evals.len(),
            domain: to_u64s(&domain),
            evaluations: to_u64s(&evals),
            merkle_root: Some(root.to_canonical_u64()),
            beta: beta.map(|b| b.to_canonical_u64()),
        });
        if evals.len() <= final_layer_size {
            break;
        }
        let (next_evals, next_domain) = fri::fold_layer(&evals, &domain, beta_seed);
        evals = next_evals;
        domain = next_domain;
        beta_seed = beta_seed * ToyField::from_u64(7) + ToyField::from_u64(1); 
        round_idx += 1;
    }

    let final_points: Vec<(ToyField, ToyField)> = domain.iter().copied().zip(evals.iter().copied()).take(1).collect();
    let final_poly_coeffs = poly::interpolate::lagrange_interpolate(&final_points).coeffs().to_vec();

    FriExport { rounds, final_poly_coeffs: to_u64s(&final_poly_coeffs) }
}

pub fn export_fri() -> FriExport {
    export_fri_rounds(ToyField::from_u64(2))
}

pub fn export_proof() -> ProofExport {
    let h = hasher();
    let start = ToyField::from_u64(START);
    let step = ToyField::from_u64(STEP);
    let end = start + step * ToyField::from_u64((TRACE_LEN - 1) as u64);

    let trace: Vec<ToyField> = (0..TRACE_LEN).map(|i| start + step * ToyField::from_u64(i as u64)).collect();
    let mut coeffs = fft_interpolate(&trace).coeffs().to_vec();
    coeffs.resize(TRACE_LEN, ToyField::zero());
    let offset = ToyField::from_u64(2);
    let lde_evals = low_degree_extend(&coeffs, BLOWUP, offset);
    let big_n = TRACE_LEN * BLOWUP;

    let trace_tree = MerkleTree::from_values(&h, &lde_evals);
    let trace_root = trace_tree.root();

    let mut transcript = Transcript::new(&h, "viz-export-arith-progression-v1");
    transcript.absorb_u64(TRACE_LEN as u64);
    transcript.absorb(start);
    transcript.absorb(step);
    transcript.absorb(trace_root);
    let alpha = transcript.squeeze_field();

    let log_n = TRACE_LEN.trailing_zeros();
    let gen = ToyField::root_of_unity(log_n);
    let last_point = gen.pow((TRACE_LEN - 1) as u64);
    let domain = coset_domain(big_n, offset);
    let composition: Vec<ToyField> = (0..big_n)
        .map(|k| {
            let x = domain[k];
            let a_here = lde_evals[k];
            let a_next = lde_evals[(k + BLOWUP) % big_n];
            let vanishing_h = x.pow(TRACE_LEN as u64) - ToyField::one();
            let trans_num = a_next - a_here - step;
            let trans_zero = vanishing_h * (x - last_point).inverse().unwrap();
            let trans_q = trans_num * trans_zero.inverse().unwrap();
            let start_q = (a_here - start) * (x - ToyField::one()).inverse().unwrap();
            let end_q = (a_here - end) * (x - last_point).inverse().unwrap();
            alpha * trans_q + alpha * alpha * start_q + alpha * alpha * alpha * end_q
        })
        .collect();

    let fri_prover = FriProver::commit(&h, composition, offset, 2, 1, &mut transcript);
    let bound = fri_prover.index_bound();
    let num_queries = 6;
    let indices: Vec<usize> = (0..num_queries).map(|_| transcript.squeeze_index(bound)).collect();
    let fri_openings = fri_prover.open(&indices);

    let queries: Vec<QueryJson> = indices
        .iter()
        .map(|&idx| {
            let path: MerklePath<ToyField> = trace_tree.open(idx);
            QueryJson {
                index: idx,
                trace_value: lde_evals[idx].to_canonical_u64(),
                trace_path: MerklePathJson { leaf_index: path.leaf_index, siblings: to_u64s(&path.siblings) },
            }
        })
        .collect();

    let mut verifier_transcript = Transcript::new(&h, "viz-export-arith-progression-v1");
    verifier_transcript.absorb_u64(TRACE_LEN as u64);
    verifier_transcript.absorb(start);
    verifier_transcript.absorb(step);
    verifier_transcript.absorb(trace_root);
    let _alpha_v = verifier_transcript.squeeze_field();
    let betas = fri::replay_commit_phase(&fri_prover.proof, &mut verifier_transcript);
    let verifier_indices: Vec<usize> = (0..num_queries).map(|_| verifier_transcript.squeeze_index(bound)).collect();
    let accepted = indices == verifier_indices
        && fri::verify_queries(&h, &fri_prover.proof, &betas, offset, big_n, 2, 1, &verifier_indices, &fri_openings);

    ProofExport {
        statement: format!("a_0={START}, a_i+1=a_i+{STEP}, trace_len={TRACE_LEN}"),
        start: start.to_canonical_u64(),
        step: step.to_canonical_u64(),
        end: end.to_canonical_u64(),
        trace_root: trace_root.to_canonical_u64(),
        num_queries,
        queries,
        fri_layer_roots: to_u64s(&fri_prover.proof.layer_roots),
        fri_final_poly_coeffs: to_u64s(&fri_prover.proof.final_poly_coeffs),
        accepted,
    }
}

