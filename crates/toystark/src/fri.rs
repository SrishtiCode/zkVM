//! FRI — the **F**ast **R**eed-Solomon **I**OP of Proximity.
//!
//! FRI is how a STARK proves "this committed set of evaluations really is
//! (close to) a low-degree polynomial" without the verifier ever seeing
//! the whole polynomial. The idea:
//!
//! 1. **Fold.** Given evaluations of `p(x)` of degree `< d` over a domain,
//!    a random linear combination of `p`'s even- and odd-degree parts
//!    produces `p'(y)` of degree `< d/2` over a domain half the size
//!    (`y = x^2`). Repeat until the polynomial is small enough to just
//!    send directly.
//! 2. **Commit.** Merkle-root each layer's evaluations before revealing
//!    the next round's folding challenge (Fiat–Shamir) — this is what
//!    stops a prover from folding dishonestly.
//! 3. **Query.** The verifier picks random domain points and checks that
//!    the folding equation actually holds there, across every layer down
//!    to the final polynomial. If the prover cheated anywhere, most
//!    random query points catch it.
//!
//! This module splits cleanly into a **commit phase** (fold + Merkle
//! roots + the final polynomial's coefficients — everything needed to
//! derive Fiat–Shamir randomness) and a **query phase** (opening/checking
//! specific domain points), because the caller (see `prover`/`verifier`)
//! needs to interleave FRI's query indices with the trace's own openings.

use crate::hash::{hash_leaf, MerklePath, MerkleTree};
use crate::transcript::Transcript;
use field::Field;
use poly::interpolate::lagrange_interpolate;
use poly::lde::coset_domain;
use poly::Polynomial;

/// A single round's worth of openings for one FRI query: the two sibling
/// values folding combines, each with a Merkle path into that round's
/// committed layer.
#[derive(Debug, Clone)]
pub struct FriRoundOpening<F: Field> {
    pub value_a: F,
    pub path_a: MerklePath,
    pub value_b: F,
    pub path_b: MerklePath,
}

/// Everything the verifier needs to check FRI's commit phase and derive
/// the same Fiat–Shamir randomness the prover used.
#[derive(Debug, Clone)]
pub struct FriCommitPhaseProof<F: Field> {
    /// One Merkle root per folded layer (not including the final layer,
    /// which is small enough to send as coefficients instead).
    pub layer_roots: Vec<u64>,
    /// Coefficients of the final, tiny polynomial.
    pub final_poly_coeffs: Vec<F>,
}

/// Prover-side state from the commit phase, kept around so [`open`] can
/// answer query challenges without re-folding from scratch.
pub struct FriProver<F: Field> {
    /// `eval_layers[r]` are the evaluations of round `r`'s (folded)
    /// polynomial; `eval_layers[0]` is the input, `eval_layers.last()` is
    /// the final small layer.
    eval_layers: Vec<Vec<F>>,
    /// One Merkle tree per committed layer (`eval_layers[0..num_rounds]`
    /// — everything except the uncommitted final layer).
    trees: Vec<MerkleTree>,
    pub proof: FriCommitPhaseProof<F>,
    initial_domain_size: usize,
    domain_offset: F,
}

fn values_to_leaf_hashes<F: Field>(values: &[F]) -> Vec<u64> {
    values.iter().map(|v| hash_leaf(v.to_canonical_u64())).collect()
}

/// Folds one layer of evaluations into the next: `evals`/`domain` of size
/// `m` become size `m/2`, using `beta` as the random folding challenge.
/// `domain[i]` and `domain[i + m/2]` must be `x` and `-x` for the same
/// `x` (true for any coset of a power-of-two subgroup), which is what
/// makes `domain[i]^2` well-defined as the corresponding point in the
/// output domain.
fn fold_layer<F: Field>(evals: &[F], domain: &[F], beta: F) -> (Vec<F>, Vec<F>) {
    let half = evals.len() / 2;
    let two_inv = F::from_u64(2).inverse().expect("2 is invertible in any prime field used here");
    let mut next_evals = Vec::with_capacity(half);
    let mut next_domain = Vec::with_capacity(half);
    for i in 0..half {
        let (a, b, x) = (evals[i], evals[i + half], domain[i]);
        let even = (a + b) * two_inv;
        let odd = (a - b) * two_inv * x.inverse().expect("domain points are nonzero");
        next_evals.push(even + beta * odd);
        next_domain.push(x * x);
    }
    (next_evals, next_domain)
}

/// The value `fold_layer` would produce for a single pair `(a, b)` sitting
/// at domain point `x`. Used by the verifier, which only ever has one
/// pair at a time (from an opened query), not a whole layer.
fn fold_value<F: Field>(a: F, b: F, x: F, beta: F) -> F {
    let two_inv = F::from_u64(2).inverse().unwrap();
    let even = (a + b) * two_inv;
    let odd = (a - b) * two_inv * x.inverse().expect("domain points are nonzero");
    even + beta * odd
}

/// The point at index `i` of the layer-`r` domain, computed directly from
/// the *initial* domain's parameters rather than by re-folding: layer
/// `r`'s domain is the initial domain raised to the `2^r`, since folding
/// squares every domain point each round.
pub(crate) fn domain_point<F: Field>(offset: F, generator: F, i: usize, r: u32) -> F {
    (offset * generator.pow(i as u64)).pow(1u64 << r)
}

impl<F: Field> FriProver<F> {
    /// Runs FRI's commit phase: repeatedly fold `initial_evals` (evaluations
    /// of the polynomial being tested, on the coset `domain_offset * H_N`)
    /// down to `final_layer_size`, Merkle-committing every layer along the
    /// way and absorbing/squeezing through `transcript` exactly as the
    /// verifier will replay it.
    pub fn commit(
        initial_evals: Vec<F>,
        domain_offset: F,
        final_layer_size: usize,
        final_poly_num_coeffs: usize,
        transcript: &mut Transcript,
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

        // Final layer: interpolate through only `final_poly_num_coeffs` of
        // its points — strictly fewer than the layer's full size. If the
        // data really did come from folding a low-degree polynomial, every
        // other point in the layer will agree with this polynomial too
        // (checked implicitly across many queries); if not, most of them
        // won't, and a query landing on a disagreement is rejected.
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

    /// Number of folding rounds (= number of Merkle-committed layers).
    pub fn num_rounds(&self) -> usize {
        self.trees.len()
    }

    /// The bound query indices must fall in: `[0, initial_domain_size / 2)`.
    pub fn index_bound(&self) -> usize {
        self.initial_domain_size / 2
    }

    /// Answers a batch of query challenges: for each starting index
    /// `idx0` (already reduced mod [`Self::index_bound`] by the caller),
    /// opens the sibling pair used at every folding round.
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

/// Verifier-side replay of FRI's commit phase: absorbs the layer roots
/// and final-polynomial coefficients from the proof in the same order
/// the prover did, recovering the same folding challenges (`betas`).
///
/// Returns the recovered `betas` (one per layer root, i.e. one per fold),
/// which [`verify_queries`] needs.
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

/// Checks a batch of query openings against a FRI commit-phase proof.
///
/// `indices` must be the same values (derived from the same transcript,
/// in the same order) the prover used to produce `openings` via
/// [`FriProver::open`].
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
        // Enforced here, not just at commit time: a cheating prover could
        // otherwise send a full-length (zero-redundancy) final polynomial
        // that trivially matches any folded data.
        return false;
    }

    let log_n = initial_domain_size.trailing_zeros();
    let generator = F::root_of_unity(log_n);
    let final_poly = Polynomial::new(proof.final_poly_coeffs.clone());

    // layer_sizes[r] = size of layer r, for r = 0..=num_rounds (the last
    // entry is the uncommitted final layer).
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

    /// Runs FRI prove + verify end to end for evaluations of a genuine
    /// low-degree polynomial, checking the proof is accepted.
    fn run_fri_roundtrip(
        coeffs: Vec<ToyField>,
        blowup: usize,
        final_layer_size: usize,
        final_poly_num_coeffs: usize,
        num_queries: usize,
    ) -> bool {
        let n = coeffs.len();
        let domain_size = n * blowup;
        let offset = tf(2); // a fixed, small coset offset outside H_domain_size
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
        // coeffs.len() = 4 (degree < 4) is our claimed bound D0. Folding
        // domain 16 -> 8 -> 4 (2 rounds) halves that bound twice too:
        // 4 -> 2 -> 1, so a *single* final coefficient is the right,
        // tightest-possible redundant check (final_layer_size=4 points
        // must all agree with one constant).
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
        // NOTE: a *single-point* flip of an otherwise-honest low-degree
        // codeword sits well within the Reed-Solomon code's unique-decoding
        // radius — FRI's proximity test is only guaranteed to reject data
        // that's genuinely *far* from every low-degree polynomial, so a
        // 1-in-16 error is legitimately allowed to pass sometimes (that's
        // expected FRI behavior, not a soundness bug in this
        // implementation; the STARK-level composition step is what turns
        // "trace disagrees with the constraint" into "far from low
        // degree," see `air::tests::tampered_trace_gives_high_degree_composition`).
        // To exercise FRI's rejection path directly, commit to data with
        // no low-degree structure at all: pseudo-random evaluations across
        // the *whole* domain, overwhelmingly far from any low-degree poly.
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
        // Note: replaying against the tampered proof changes the
        // transcript state (different bytes absorbed), so in a full
        // protocol this alone could change query indices too; here we
        // reuse the prover's indices directly to isolate the final-poly
        // check specifically.
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
        // A cheating prover sending a *full-length* (zero-redundancy)
        // final polynomial should be caught even if every individual
        // value happens to be "correct" — the count itself is checked.
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
        // Pad with the honest additional coefficients it would have had if
        // interpolated through all 4 final-layer points instead of 1 — a
        // strictly more informative (zero-redundancy) polynomial.
        oversized_proof.final_poly_coeffs.resize(final_layer_size, ToyField::from_u64(0));

        let mut verifier_transcript = Transcript::new(&[domain_size as u64]);
        let betas = replay_commit_phase(&oversized_proof, &mut verifier_transcript);

        let accepted = verify_queries(
            &oversized_proof,
            &betas,
            offset,
            domain_size,
            final_layer_size,
            1, // verifier still expects the agreed-upon count
            &indices,
            &openings,
        );
        assert!(!accepted);
    }
}
