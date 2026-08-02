//! Low-Degree Extension (LDE).
//!
//! A STARK prover begins with a polynomial defined over a small evaluation
//! domain (typically the execution trace domain).
//!
//! The Low-Degree Extension (LDE) evaluates **the same polynomial** over a
//! much larger domain.
//!
//! The polynomial itself does not change.
//! Only the number of evaluation points increases.
//!
//! Example
//! -------
//!
//! Original domain (size 8):
//!
//! ω⁰ ω¹ ω² ... ω⁷
//!
//! ↓
//!
//! Blowup factor = 4
//!
//! ↓
//!
//! Extended domain (size 32):
//!
//! Ω⁰ Ω¹ Ω² ... Ω³¹
//!
//! where Ω is a primitive 32nd root of unity.
//!
//! These additional evaluations are later committed with a Merkle tree and
//! checked using FRI.

use crate::fft;
use field::Field;

pub fn low_degree_extend<F: Field>(coeffs: &[F], blowup_factor: usize, offset: F) -> Vec<F> {
    assert!(
        coeffs.len().is_power_of_two(),
        "low_degree_extend requires a power-of-two number of coefficients, got {}",
        coeffs.len()
    );
    assert!(
        blowup_factor.is_power_of_two() && blowup_factor >= 1,
        "blowup_factor must be a power of two >= 1, got {blowup_factor}"
    );
    assert!(!offset.is_zero(), "coset offset must be nonzero");

    let n = coeffs.len();
    let big_n = n * blowup_factor;

    let mut padded = vec![F::zero(); big_n];
    let mut offset_pow = F::one();
    for (i, &c) in coeffs.iter().enumerate() {
        padded[i] = c * offset_pow;
        offset_pow *= offset;
    }

    fft::fft(&mut padded);
    padded
}

pub fn coset_domain<F: Field>(size: usize, offset: F) -> Vec<F> {
    assert!(size.is_power_of_two(), "domain size must be a power of two, got {size}");
    let log_n = size.trailing_zeros();
    let generator = F::root_of_unity(log_n);
    let mut domain = Vec::with_capacity(size);
    let mut x = offset;
    for _ in 0..size {
        domain.push(x);
        x *= generator;
    }
    domain
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Polynomial;
    use field::ToyField;

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    #[test]
    fn lde_matches_direct_evaluation_on_the_coset() {
        // p(x) = 1 + 2x + 3x^2 + 4x^3, natural domain size 4, blow up to 16.
        let coeffs = vec![tf(1), tf(2), tf(3), tf(4)];
        let poly = Polynomial::new(coeffs.clone());
        let blowup = 4;
        let offset = tf(2); // must lie outside H_16 to be a genuine coset; 2 works here.

        let evals = low_degree_extend(&coeffs, blowup, offset);
        let domain = coset_domain(coeffs.len() * blowup, offset);

        assert_eq!(evals.len(), 16);
        for (k, &x) in domain.iter().enumerate() {
            assert_eq!(evals[k], poly.eval(x), "mismatch at coset point {k}");
        }
    }

    #[test]
    fn blowup_factor_one_is_plain_evaluation_on_h_n() {
        let coeffs = vec![tf(5), tf(1)]; // p(x) = 5 + x
        let evals = low_degree_extend(&coeffs, 1, tf(1));
        let domain = coset_domain(2, tf(1));
        assert_eq!(evals[0], tf(5) + domain[0]);
        assert_eq!(evals[1], tf(5) + domain[1]);
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn non_power_of_two_blowup_panics() {
        let coeffs = vec![tf(1), tf(2)];
        let _ = low_degree_extend(&coeffs, 3, tf(2));
    }

    #[test]
    #[should_panic(expected = "nonzero")]
    fn zero_offset_panics() {
        let coeffs = vec![tf(1), tf(2)];
        let _ = low_degree_extend(&coeffs, 2, tf(0));
    }
}
