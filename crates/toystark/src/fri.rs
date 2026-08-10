use crate::hash::{hash_leaf, MerklePath, MerkleTree};
use crate::transcript::Transcript;
use field::Field;
use poly::interpolate::lagrange_interpolate;
use poly::lde::coset_domain;
use poly::Polynomial;

#[derive(Debug, Clone)]
pub struct FriRoundOpening<F: Field>{
    pub value_a: F,
    pub path_a: MerklePath,
    pub value_b: F,
    pub path_b: MerklePath,       
} 

#[derive(Debug, Clone)]
pub struct FriCommitPhaseProof<F: Field> {
    pub layer_roots: Vec<u64>,
    pub final_poly_coeffs: Vec<F>,
}

pub struct FriProver<F: Field>{
    eval_layers: Vec<Vec<F>>,
    trees: Vec<MerkleTree>,
    pub proof: FriCommitPhaseProof<F>,
    initial_domain_size: usize,
    domain_offset: F,      
}   

fn values_to_leaf_hashes<F: Field>(values: &[F]) -> Vec<u64>{
    values
        .iter()
        .map(|v| hash_leaf(v.to_canonical_u64()))
        .collect()
} 

fn fold_layer<F:Field>(evals: &[F], domain: &[F], beta: F) -> (Vec<F>, Vec<F>){
    let half = evals.len()/2;
    let two_inv = F::from_u64(2).inverse().expect("field characteristic must not be 2");
    let mut next_evals = Vec::with_capacity(half);
    let mut next_domain = Vec::with_capacity(half);
    for i in 0..half{
        let (a,b,x) = (evals[i], evals[i+half], domain[i]);
        let even = (a + b) * two_inv;
        let odd = (a - b) * two_inv * x.inverse().expect("domain points are nonzero");
        next_evals.push(even + beta * odd);
        next_domain.push(x * x);     
    }          
    (next_evals, next_domain)
}  

fn fold_value<F: Field>(a: F, b: F, x: F, beta: F) -> F{
    let two_inv = F::from_u64(2).inverse().unwrap();
    let even = (a+b)*two_inv;
    let odd = (a-b)*two_inv*x.inverse().expect("domain points are nonzero");
    even + beta * odd     
}    

pub(crate) fn domain_point<F: Field>(offset: F, generator: F, i: usize, r: u32) -> F {
    (offset * generator.pow(i as u64)).pow(1u64 << r)
}

impl <F: Field> FriProver<F>{
    pub fn commit(
        initial_evals: Vec<F>,
        domain_offset: F,
        final_layer_size: usize,
        final_poly_num_coeffs: usize,
        transcript: &mut Transcript,  
    ) -> Self{
        let initial_domain_size = initial_evals.len();
        assert!(initial_domain_size.is_power_of_two(), "FRI domain must be a power of two");
        assert!(
            final_layer_size.is_power_of_two() && final_layer_size >= 1,
            "final layer size must be a power of two >= 1"
        );
        assert!(initial_domain_size >= final_layer_size);
        assert!(
            final_poly_num_coeffs.is_power_of_two() && final_poly_num_coeffs >= 1,
            "final polynomial coefficient count must be a power of two >= 1"
        );
        assert!(
            final_poly_num_coeffs < final_layer_size,
            "final polynomial must have strictly fewer coefficients ({final_poly_num_coeffs}) than \
             final-layer points ({final_layer_size}) — otherwise the last FRI check has zero \
             redundancy and trivially accepts *any* data, honest or not"
        );

        let mut domain = coset_domain(initial_domain_size, domain_offset);
        let mut evals = initial_evals;

        let mut eval_layers = vec![evals.clone()];
        let mut trees = Vec::new();
        let mut layer_roots = Vec::new();

        let first_tree = MerkleTree::build(&values_to_leaf_hashes(&evals));
        layer_roots.push(first_tree.root());
        transcript.absorb(first_tree.root());
        trees.push(first_tree);

        while evals.len() > final_layer_size {
            let beta = transcript.squeeze_field::<F>();
            let (next_evals, next_domain) = fold_layer(&evals, &domain, beta);
            evals = next_evals;
            domain = next_domain;
            eval_layers.push(evals.clone());

            if evals.len() > final_layer_size {
                let tree = MerkleTree::build(&values_to_leaf_hashes(&evals));
                layer_roots.push(tree.root());
                transcript.absorb(tree.root());
                trees.push(tree);
            }
        }

        let final_points: Vec<(F, F)> =
            domain.iter().copied().zip(evals.iter().copied()).take(final_poly_num_coeffs).collect();
        let final_poly_coeffs = lagrange_interpolate(&final_points).coeffs().to_vec();
        for &c in &final_poly_coeffs {
            transcript.absorb_field(c);
        }

        FriProver {
            eval_layers,
            trees,
            proof: FriCommitPhaseProof { layer_roots, final_poly_coeffs },
            initial_domain_size,
            domain_offset,
        }
    }

    pub fn num_rounds(&self) -> usize{
        self.trees.len()
    } 

    pub fn index_bound(&self) -> usize{
        self.initial_domain_size / 2
    }

    pub fn open(&self, indices: &[usize]) -> Vec<Vec<FriRoundOpening<F>>> {
        let num_rounds = self.num_rounds();
        indices
            .iter()
            .map(|&idx0| {
                let mut layer_size = self.initial_domain_size;
                (0..num_rounds)
                    .map(|r| {
                        let half = layer_size / 2;
                        let i = idx0 % half;
                        let value_a = self.eval_layers[r][i];
                        let value_b = self.eval_layers[r][i + half];
                        let path_a = self.trees[r].open(i);
                        let path_b = self.trees[r].open(i + half);
                        layer_size = half;
                        FriRoundOpening { value_a, path_a, value_b, path_b }
                    })
                    .collect()
            })
            .collect()
    }

    pub fn domain_offset(&self) -> F {
        self.domain_offset
    }
}

pub fn replay_commit_phase<F: Field>(
    proof: &FriCommitPhaseProof<F>,
    transcript: &mut Transcript,
) -> Vec<F> {
    let mut betas = Vec::with_capacity(proof.layer_roots.len());
    for &root in &proof.layer_roots {
        transcript.absorb(root);
        betas.push(transcript.squeeze_field::<F>());
    }
    for &c in &proof.final_poly_coeffs {
        transcript.absorb_field(c);
    }
    betas
}

#[allow(clippy::too_many_arguments)]
pub fn verify_queries<F: Field>(
    proof: &FriCommitPhaseProof<F>,
    betas: &[F],
    domain_offset: F,
    initial_domain_size: usize,
    final_layer_size: usize,
    expected_final_poly_num_coeffs: usize,
    indices: &[usize],
    openings: &[Vec<FriRoundOpening<F>>],
) -> bool {
    let num_rounds = proof.layer_roots.len();
    if betas.len() != num_rounds || indices.len() != openings.len() {
        return false;
    }
    if proof.final_poly_coeffs.len() != expected_final_poly_num_coeffs {
        return false;
    }

    let log_n = initial_domain_size.trailing_zeros();
    let generator = F::root_of_unity(log_n);
    let final_poly = Polynomial::new(proof.final_poly_coeffs.clone());

    let mut layer_sizes = vec![initial_domain_size];
    for _ in 0..num_rounds {
        layer_sizes.push(layer_sizes.last().unwrap() / 2);
    }
    if *layer_sizes.last().unwrap() != final_layer_size {
        return false;
    }

    for (rounds, &idx0) in openings.iter().zip(indices) {
        if rounds.len() != num_rounds {
            return false;
        }
        for (r, opening) in rounds.iter().enumerate() {
            let layer_size = layer_sizes[r];
            let half = layer_size / 2;
            let i = idx0 % half;

            if opening.path_a.leaf_index != i || opening.path_b.leaf_index != i + half {
                return false;
            }
            if !opening.path_a.verify(proof.layer_roots[r], hash_leaf(opening.value_a.to_canonical_u64())) {
                return false;
            }
            if !opening.path_b.verify(proof.layer_roots[r], hash_leaf(opening.value_b.to_canonical_u64())) {
                return false;
            }

            let x = domain_point(domain_offset, generator, i, r as u32);
            let folded = fold_value(opening.value_a, opening.value_b, x, betas[r]);

            let next_layer_size = layer_sizes[r + 1];
            let next_half = next_layer_size / 2;
            let next_index_in_layer = idx0 % next_layer_size;

            if r + 1 < num_rounds {
                let expected = if next_index_in_layer < next_half {
                    rounds[r + 1].value_a
                } else {
                    rounds[r + 1].value_b
                };
                if folded != expected {
                    return false;
                }
            } else {
                let final_x = domain_point(domain_offset, generator, next_index_in_layer, (r + 1) as u32);
                if final_poly.eval(final_x) != folded {
                    return false;
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;
    use poly::lde::low_degree_extend;

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
        let n = coeffs.len();
        let domain_size = n * blowup;
        let offset = tf(2); 
        let evals = low_degree_extend(&coeffs, blowup, offset);

        let mut prover_transcript = Transcript::new(&[domain_size as u64]);
        let prover =
            FriProver::commit(evals, offset, final_layer_size, final_poly_num_coeffs, &mut prover_transcript);
        let bound = prover.index_bound();
        let indices: Vec<usize> = (0..num_queries).map(|_| prover_transcript.squeeze_index(bound)).collect();
        let openings = prover.open(&indices);
        let commit_proof = prover.proof.clone();

        let mut verifier_transcript = Transcript::new(&[domain_size as u64]);
        let betas = replay_commit_phase(&commit_proof, &mut verifier_transcript);
        let verifier_bound = domain_size / 2;
        let verifier_indices: Vec<usize> =
            (0..num_queries).map(|_| verifier_transcript.squeeze_index(verifier_bound)).collect();
        assert_eq!(indices, verifier_indices, "prover/verifier index derivation diverged");

        verify_queries(
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
    fn rejects_data_with_no_low_degree_structure() {
        let domain_size = 16usize;
        let final_layer_size = 4;
        let final_poly_num_coeffs = 1;
        let offset = tf(2);
        let evals: Vec<ToyField> = (0..domain_size as u64).map(|i| tf(hash_leaf(i))).collect();

        let mut prover_transcript = Transcript::new(&[domain_size as u64]);
        let prover = FriProver::commit(
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
        let commit_proof = prover.proof.clone();

        let mut verifier_transcript = Transcript::new(&[domain_size as u64]);
        let betas = replay_commit_phase(&commit_proof, &mut verifier_transcript);
        let verifier_indices: Vec<usize> =
            (0..num_queries).map(|_| verifier_transcript.squeeze_index(domain_size / 2)).collect();

        let accepted = verify_queries(
            &commit_proof,
            &betas,
            offset,
            domain_size,
            final_layer_size,
            final_poly_num_coeffs,
            &verifier_indices,
            &openings,
        );
        assert!(!accepted, "data with no low-degree structure should be caught with high probability");
    }

    #[test]
    fn rejects_wrong_final_polynomial() {
        let coeffs: Vec<ToyField> = vec![1, 2, 3, 4].into_iter().map(tf).collect();
        let domain_size = coeffs.len() * 4;
        let final_layer_size = 4;
        let final_poly_num_coeffs = 1;
        let offset = tf(2);
        let evals = low_degree_extend(&coeffs, 4, offset);

        let mut prover_transcript = Transcript::new(&[domain_size as u64]);
        let prover = FriProver::commit(
            evals,
            offset,
            final_layer_size,
            final_poly_num_coeffs,
            &mut prover_transcript,
        );
        let bound = prover.index_bound();
        let indices: Vec<usize> = (0..10).map(|_| prover_transcript.squeeze_index(bound)).collect();
        let openings = prover.open(&indices);
        let mut tampered_proof = prover.proof.clone();
        tampered_proof.final_poly_coeffs[0] += tf(1);

        let mut verifier_transcript = Transcript::new(&[domain_size as u64]);
        let betas = replay_commit_phase(&tampered_proof, &mut verifier_transcript);

        let accepted = verify_queries(
            &tampered_proof,
            &betas,
            offset,
            domain_size,
            final_layer_size,
            final_poly_num_coeffs,
            &indices,
            &openings,
        );
        assert!(!accepted);
    }

    #[test]
    fn rejects_final_polynomial_with_wrong_coefficient_count() {
        let coeffs: Vec<ToyField> = vec![1, 2, 3, 4].into_iter().map(tf).collect();
        let domain_size = coeffs.len() * 4;
        let final_layer_size = 4;
        let offset = tf(2);
        let evals = low_degree_extend(&coeffs, 4, offset);

        let mut prover_transcript = Transcript::new(&[domain_size as u64]);
        let prover = FriProver::commit(evals, offset, final_layer_size, 1, &mut prover_transcript);
        let bound = prover.index_bound();
        let indices: Vec<usize> = (0..10).map(|_| prover_transcript.squeeze_index(bound)).collect();
        let openings = prover.open(&indices);

        let mut oversized_proof = prover.proof.clone();
        oversized_proof.final_poly_coeffs.resize(final_layer_size, ToyField::from_u64(0));

        let mut verifier_transcript = Transcript::new(&[domain_size as u64]);
        let betas = replay_commit_phase(&oversized_proof, &mut verifier_transcript);

        let accepted = verify_queries(
            &oversized_proof,
            &betas,
            offset,
            domain_size,
            final_layer_size,
            1, 
            &indices,
            &openings,
        );
        assert!(!accepted);
    }
}
