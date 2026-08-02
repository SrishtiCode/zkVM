/*
Points / evaluations
       │
       ├── Lagrange interpolation ──→ works for arbitrary x values
       │                              O(n²)
       │
       └── FFT interpolation ───────→ x values must be roots of unity
                                      O(n log n)

Points
(0,1)
(1,2)
(2,5)

   │
   │ interpolation
   ▼

P(x) = 1 + x²

coefficients = [1,0,1]   

Trace values
    │
    ▼
[3, 7, 11, 15]
    │
 interpolation
    ▼
Trace polynomial T(x)
*/

use crate::fft;
use crate::Polynomial;
use field::Field;

pub fn lagrange_interpolate<F: Field>(points: &[(F, F)]) -> Polynomial<F>{
    assert!(!points.is_empty(), "cannot interpolate through zero points");

    for i in 0..points.len(){
        for j in (i+1)..points.len(){
            assert!(
                points[i].0 != points[j].0,
                "duplicate x-coordinate in interpolation points"
            );
        }
    }

    let mut result = Polynomial::zero();

    for (i, &(xi, yi)) in points.iter().enumerate(){
        let mut numerator = Polynomial::new(vec![F::one()]);
        let mut denominator = F::one();
        for (j, &(xj, _)) in points.iter().enumerate(){
            if i == j {
                continue;
            }
            numerator = numerator.mul_naive(&Polynomial::new(vec![-xj, F::one()]));
            denominator *= xi - xj;
        }
        let scale = yi
            * denominator
                .inverse()
                .expect("distinct x-coordinates checked above");
        result = result.add(&numerator.scale(scale));        
    }
    result
}  

pub fn fft_interpolate<F: Field>(evals: &[F]) -> Polynomial<F> {
    assert!(
        evals.len().is_power_of_two(),
        "fft_interpolate requires a power-of-two number of evaluations, got {}",
        evals.len()
    );
    let mut coeffs = evals.to_vec();
    fft::ifft(&mut coeffs);
    Polynomial::new(coeffs)
}


#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    #[test]
    fn lagrange_reproduces_known_polynomial() {
        // p(x) = x^2 + 1 -> p(0)=1, p(1)=2, p(2)=5
        let points = [(tf(0), tf(1)), (tf(1), tf(2)), (tf(2), tf(5))];
        let p = lagrange_interpolate(&points);
        for &(x, y) in &points {
            assert_eq!(p.eval(x), y);
        }
        assert_eq!(p.coeffs(), &[tf(1), tf(0), tf(1)]);
    }

    #[test]
    fn lagrange_handles_single_point_as_constant() {
        let points = [(tf(3), tf(9))];
        let p = lagrange_interpolate(&points);
        assert_eq!(p.eval(tf(3)), tf(9));
        assert_eq!(p.eval(tf(100)), tf(9)); // constant everywhere
    }

    #[test]
    #[should_panic(expected = "duplicate x-coordinate")]
    fn lagrange_rejects_duplicate_x() {
        let points = [(tf(1), tf(2)), (tf(1), tf(5))];
        let _ = lagrange_interpolate(&points);
    }

    #[test]
    fn fft_interpolate_matches_lagrange_on_roots_of_unity() {
        let log_n = 3u32;
        let n = 1usize << log_n;
        let omega = ToyField::root_of_unity(log_n);

        // an arbitrary degree-<n polynomial to round-trip through
        let original = Polynomial::new((1..=n as u64).map(tf).collect());

        let mut x = ToyField::one();
        let mut points = Vec::with_capacity(n);
        let mut evals = Vec::with_capacity(n);
        for _ in 0..n {
            evals.push(original.eval(x));
            points.push((x, original.eval(x)));
            x *= omega;
        }

        let via_fft = fft_interpolate(&evals);
        let via_lagrange = lagrange_interpolate(&points);

        assert_eq!(via_fft, original);
        assert_eq!(via_fft, via_lagrange);
    }
}
