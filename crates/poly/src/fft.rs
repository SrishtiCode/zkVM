//It implements FFT (Fast Fourier Transform) and IFFT (Inverse Fast Fourier Transform) over your finite fields.

//FFT : coeeficients -> evaluations
//IFFT : evaluations -> coefficients

// Suppose P(x) = 1 + 2x + 3x^2 + 4x^3
// Coefficents : [1,2,3,4]
// [1,2,3,4] -> FFT -> [P(1), P(ω), P(ω²), P(ω³)]

use field::Field;

fn reverse_bits(mut x: usize, bits: u32) -> usize{
    let mut result = 0usize;
    for _ in 0..bits{
        result = (result << 1) | (x & 1);
        x >>= 1;
    }
    result
}

fn bit_reverse_permute<F: Field>(values: &mut [F]){
    let n = values.len();
    let bits = n.trailing_zeros();
    for i in 0..n{
        let j = reverse_bits(i, bits);
        if i < j {
            values.swap(i,j);
        } 
    }
}

pub fn fft<F: Field>(values: &mut [F]) {
    let n = values.len();
    assert!(
        n.is_power_of_two(),
        "fft domain size must be a power of two, got {n}"
    );
    if n <= 1 {
        return;
    }
    let log_n = n.trailing_zeros();

    bit_reverse_permute(values);

    for s in 1..=log_n {
        let m = 1usize << s;
        let half_m = m / 2;
        let w_m = F::root_of_unity(s);
        for block_start in (0..n).step_by(m) {
            let mut w = F::one();
            for j in 0..half_m {
                let u = values[block_start + j];
                let t = w * values[block_start + j + half_m];
                values[block_start + j] = u + t;
                values[block_start + j + half_m] = u - t;
                w *= w_m;
            }
        }
    }
}

pub fn ifft<F: Field>(values: &mut [F]) {
    let n = values.len();
    assert!(
        n.is_power_of_two(),
        "ifft domain size must be a power of two, got {n}"
    );
    if n <= 1 {
        return;
    }
    let log_n = n.trailing_zeros();

    bit_reverse_permute(values);

    for s in 1..=log_n {
        let m = 1usize << s;
        let half_m = m / 2;
        let w_m = F::root_of_unity(s)
            .inverse()
            .expect("root of unity is nonzero, so it is invertible");
        for block_start in (0..n).step_by(m) {
            let mut w = F::one();
            for j in 0..half_m {
                let u = values[block_start + j];
                let t = w * values[block_start + j + half_m];
                values[block_start + j] = u + t;
                values[block_start + j + half_m] = u - t;
                w *= w_m;
            }
        }
    }

    let n_inv = F::from_u64(n as u64)
        .inverse()
        .expect("n is a power of two and the field's characteristic is odd, so it is invertible");
    for v in values.iter_mut() {
        *v *= n_inv;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use field::{Field, Goldilocks, ToyField};

    fn naive_dft<F: Field>(coeffs: &[F]) -> Vec<F> {
        let n = coeffs.len();
        let log_n = n.trailing_zeros();
        let omega = F::root_of_unity(log_n);
        let mut out = vec![F::zero(); n];
        for (k, slot) in out.iter_mut().enumerate() {
            let x = omega.pow(k as u64);
            let mut acc = F::zero();
            for &c in coeffs.iter().rev() {
                acc = acc * x + c;
            }
            *slot = acc;
        }
        out
    }

    #[test]
    fn fft_matches_naive_dft_toyfield() {
        let coeffs: Vec<ToyField> = (1..=8).map(ToyField::from_u64).collect();
        let expected = naive_dft(&coeffs);
        let mut actual = coeffs.clone();
        fft(&mut actual);
        assert_eq!(actual, expected);
    }

    #[test]
    fn fft_matches_naive_dft_goldilocks() {
        let coeffs: Vec<Goldilocks> = (1..=16).map(Goldilocks::from_u64).collect();
        let expected = naive_dft(&coeffs);
        let mut actual = coeffs.clone();
        fft(&mut actual);
        assert_eq!(actual, expected);
    }

    #[test]
    fn ifft_undoes_fft() {
        let original: Vec<ToyField> = vec![3, 1, 4, 1, 5, 9, 2, 6]
            .into_iter()
            .map(ToyField::from_u64)
            .collect();
        let mut roundtrip = original.clone();
        fft(&mut roundtrip);
        ifft(&mut roundtrip);
        assert_eq!(roundtrip, original);
    }

    #[test]
    fn fft_of_constant_polynomial_is_constant_everywhere() {
        let coeffs = vec![ToyField::from_u64(7); 1]
            .into_iter()
            .chain(std::iter::repeat_n(ToyField::zero(), 7))
            .collect::<Vec<_>>();
        let mut evals = coeffs.clone();
        fft(&mut evals);
        assert!(evals.iter().all(|&v| v == ToyField::from_u64(7)));
    }

    #[test]
    fn single_element_domain_is_identity() {
        let mut v = vec![ToyField::from_u64(42)];
        fft(&mut v);
        assert_eq!(v, vec![ToyField::from_u64(42)]);
        ifft(&mut v);
        assert_eq!(v, vec![ToyField::from_u64(42)]);
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn non_power_of_two_len_panics() {
        let mut v: Vec<ToyField> = (1..=6).map(ToyField::from_u64).collect();
        fft(&mut v);
    }
}
