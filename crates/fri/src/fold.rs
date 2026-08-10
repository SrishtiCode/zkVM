use field::Field;

pub fn fold_layer<F: Field>(evals: &[F], domain: &[F], beta: F) -> (Vec<F>, Vec<F>) {
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

pub fn fold_value<F: Field>(a: F, b: F, x: F, beta: F) -> F {
    let two_inv = F::from_u64(2).inverse().expect("2 is invertible in any prime field used here");
    let even = (a + b) * two_inv;
    let odd = (a - b) * two_inv * x.inverse().expect("domain points are nonzero");
    even + beta * odd
}  

pub fn domain_point<F: Field>(offset: F, generator: F, i: usize, r: u32) -> F {
    (offset * generator.pow(i as u64)).pow(1u64 << r)
}

//what the fuck
#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;
    use poly::lde::{coset_domain, low_degree_extend};
    use poly::Polynomial;

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    #[test]
    fn folding_a_genuine_low_degree_polynomial_matches_direct_evaluation() {
        let coeffs = vec![tf(1), tf(2), tf(3), tf(4)];
        let n = coeffs.len();
        let blowup = 4;
        let offset = tf(2);
        let evals = low_degree_extend(&coeffs, blowup, offset);
        let domain = coset_domain(n * blowup, offset);

        let beta = tf(7);
        let (folded_evals, folded_domain) = fold_layer(&evals, &domain, beta);

        let p_even = Polynomial::new(vec![tf(1), tf(3)]);
        let p_odd = Polynomial::new(vec![tf(2), tf(4)]);
        for (k, &y) in folded_domain.iter().enumerate() {
            let expected = p_even.eval(y) + beta * p_odd.eval(y);
            assert_eq!(folded_evals[k], expected, "mismatch at folded index {k}");
        }
    }

    #[test]
    fn fold_value_matches_fold_layer_pointwise() {
        let coeffs = vec![tf(5), tf(1), tf(9), tf(2)];
        let n = coeffs.len();
        let blowup = 2;
        let offset = tf(3);
        let evals = low_degree_extend(&coeffs, blowup, offset);
        let domain = coset_domain(n * blowup, offset);
        let beta = tf(6);

        let (folded_evals, _) = fold_layer(&evals, &domain, beta);
        let half = evals.len() / 2;
        for i in 0..half {
            let v = fold_value(evals[i], evals[i + half], domain[i], beta);
            assert_eq!(v, folded_evals[i]);
        }
    }

    #[test]
    fn domain_point_matches_repeated_folding_of_the_domain() {
        let offset = tf(3);
        let size = 16;
        let domain0 = coset_domain(size, offset);
        let generator = ToyField::root_of_unity(size.trailing_zeros());
        let mut domain = domain0.clone();
        for r in 0..3u32 {
            for (i, &x) in domain.iter().enumerate() {
                assert_eq!(domain_point(offset, generator, i, r), x, "mismatch at round {r}, index {i}");
            }
            let half = domain.len() / 2;
            let mut next = Vec::with_capacity(half);
            for i in 0..half {
                next.push(domain[i] * domain[i]);
            }
            domain = next;
        }
    }
}
