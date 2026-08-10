use field::Field;
use merkle::{MerklePath, MerkleTree, Poseidon};
use poly::interpolate::lagrange_interpolate;
use poly::lde::coset_domain;
use poly::Polynomial;

use crate::fold::{domain_point, fold_layer, fold_value};

pub trait ChallengeSource<F: Field> {
    fn absorb(&mut self, value: F);
    fn squeeze_field(&mut self) -> F;
    fn squeeze_index(&mut self, bound: usize) -> usize;
}

#[derive(Debug, Clone)]
pub struct FriRoundOpening<F: Field> {
    pub value_a: F,
    pub path_a: MerklePath<F>,
    pub value_b: F,
    pub path_b: MerklePath<F>,
}

#[derive(Debug, Clone)]
pub struct FriCommitPhaseProof<F: Field> {
    pub layer_roots: Vec<F>,
    pub final_poly_coeffs: Vec<F>,
}

pub struct FriProver<'h, F: Field> {
    #[allow(dead_code)]
    hasher: &'h Poseidon<F>,
    eval_layers: Vec<Vec<F>>,
    trees: Vec<MerkleTree<F>>,
    pub proof: FriCommitPhaseProof<F>,
    initial_domain_size: usize,
    domain_offset: F,
}

impl<'h, F: Field> FriProver<'h, F> {
        pub fn commit<C: ChallengeSource<F>>(
        hasher: &'h Poseidon<F>,
        initial_evals: Vec<F>,
        domain_offset: F,
        final_layer_size: usize,
        final_poly_num_coeffs: usize,
        challenges: &mut C,
    ) -> Self {
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

        let first_tree = MerkleTree::from_values(hasher, &evals);
        layer_roots.push(first_tree.root());
        challenges.absorb(first_tree.root());
        trees.push(first_tree);

        while evals.len() > final_layer_size {
            let beta = challenges.squeeze_field();
            let (next_evals, next_domain) = fold_layer(&evals, &domain, beta);
            evals = next_evals;
            domain = next_domain;
            eval_layers.push(evals.clone());

            if evals.len() > final_layer_size {
                let tree = MerkleTree::from_values(hasher, &evals);
                layer_roots.push(tree.root());
                challenges.absorb(tree.root());
                trees.push(tree);
            }
        }

                let final_points: Vec<(F, F)> =
            domain.iter().copied().zip(evals.iter().copied()).take(final_poly_num_coeffs).collect();
        let final_poly_coeffs = lagrange_interpolate(&final_points).coeffs().to_vec();
        for &c in &final_poly_coeffs {
            challenges.absorb(c);
        }

        FriProver {
            hasher,
            eval_layers,
            trees,
            proof: FriCommitPhaseProof { layer_roots, final_poly_coeffs },
            initial_domain_size,
            domain_offset,
        }
    }

    pub fn num_rounds(&self) -> usize {
        self.trees.len()
    }

    pub fn index_bound(&self) -> usize {
        self.initial_domain_size / 2
    }

    pub fn domain_offset(&self) -> F {
        self.domain_offset
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

}

pub fn replay_commit_phase<F: Field, C: ChallengeSource<F>>(
    proof: &FriCommitPhaseProof<F>,
    challenges: &mut C,
) -> Vec<F> {
    let mut betas = Vec::with_capacity(proof.layer_roots.len());
    for &root in &proof.layer_roots {
        challenges.absorb(root);
        betas.push(challenges.squeeze_field());
    }
    for &c in &proof.final_poly_coeffs {
        challenges.absorb(c);
    }
    betas
}

#[allow(clippy::too_many_arguments)]
pub fn verify_queries<F: Field>(
    hasher: &Poseidon<F>,
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
            if !opening.path_a.verify(hasher, proof.layer_roots[r], merkle::hash_leaf(hasher, opening.value_a)) {
                return false;
            }
            if !opening.path_b.verify(hasher, proof.layer_roots[r], merkle::hash_leaf(hasher, opening.value_b)) {
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
