//This file defines the main polynomial data structure and basic polynomial arithmetic              

pub mod fft;
pub mod interpolate;
pub mod lde;

use field::Field;
use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub struct Polynomial<F: Field>{
    coeffs: Vec<F>,
} 

impl<F: Field> Polynomial<F>{
    pub fn new(mut coeffs: Vec<F>) -> Self{//create polynomial
        while coeffs.len()>1 && coeffs.last() == Some(&F::zero()){
            coeffs.pop();
        }  
        if coeffs.is_empty(){
            coeffs.push(F::zero());   
        }
        Polynomial{coeffs}
    }      

    pub fn zero() -> Self{// create P(x) = 0
        Polynomial{
            coeffs: vec![F::zero()],
        }
    } 

    pub fn is_zero(&self) -> bool{//check P(x) = 0
        self.coeffs.iter().all(|c| c.is_zero())
    }    

    pub fn coeffs(&self) -> &[F] {//read coefficients
        &self.coeffs
    }

    pub fn degree(&self) -> usize {//get highest power
        self.coeffs.len() - 1
    }

    pub fn eval(&self, x: F) -> F {// calculate P(x)
        let mut acc = F::zero();
        for &c in self.coeffs.iter().rev() {
            acc = acc * x + c;
        }
        acc
    }

    pub fn add(&self, rhs: &Self) -> Self{
        let n = self.coeffs.len().max(rhs.coeffs.len());
        let mut out = vec![F::zero(); n];
        for (i, c) in self.coeffs.iter().enumerate(){
            out[i] += *c; 
        }        
        for (i, c) in rhs.coeffs.iter().enumerate(){
            out[i] += *c;  
        }
        Polynomial::new(out)
    }  

    pub fn sub(&self, rhs: &Self) -> Self {
        let n = self.coeffs.len().max(rhs.coeffs.len());
        let mut out = vec![F::zero(); n];
        for (i, c) in self.coeffs.iter().enumerate() {
            out[i] += *c;
        }
        for (i, c) in rhs.coeffs.iter().enumerate() {
            out[i] -= *c;
        }
        Polynomial::new(out)
    }
    
    pub fn mul_naive(&self, rhs: &Self) -> Self {
        if self.is_zero() || rhs.is_zero() {
            return Polynomial::zero();
        }
        let mut out = vec![F::zero(); self.coeffs.len() + rhs.coeffs.len() - 1];
        for (i, &a) in self.coeffs.iter().enumerate() {
            if a.is_zero() {
                continue;
            }
            for (j, &b) in rhs.coeffs.iter().enumerate() {
                out[i + j] += a * b;
            }
        }
        Polynomial::new(out)
    }

    pub fn mul_fft(&self, rhs: &Self) -> Self {
        if self.is_zero() || rhs.is_zero() {
            return Polynomial::zero();
        }
        let result_len = self.coeffs.len() + rhs.coeffs.len() - 1;
        let n = result_len.next_power_of_two();

        let mut a = self.coeffs.clone();
        a.resize(n, F::zero());
        let mut b = rhs.coeffs.clone();
        b.resize(n, F::zero());

        fft::fft(&mut a);
        fft::fft(&mut b);
        for i in 0..n {
            a[i] *= b[i];
        }
        fft::ifft(&mut a);

        a.truncate(result_len);
        Polynomial::new(a)
    }

    /// Scales every coefficient by `scalar`.
    pub fn scale(&self, scalar: F) -> Self {
        Polynomial::new(self.coeffs.iter().map(|&c| c * scalar).collect())
    }    
}

impl<F: Field> fmt::Debug for Polynomial<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Polynomial{:?}", self.coeffs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    fn poly(vals: &[u64]) -> Polynomial<ToyField> {
        Polynomial::new(vals.iter().map(|&v| tf(v)).collect())
    }

    #[test]
    fn trims_trailing_zero_coeffs() {
        let p = poly(&[1, 2, 0, 0]);
        assert_eq!(p.degree(), 1);
        assert_eq!(p.coeffs(), &[tf(1), tf(2)]);
    }

    #[test]
    fn eval_matches_hand_computation() {
        // p(x) = 3 + 2x + x^2, p(5) = 3 + 10 + 25 = 38
        let p = poly(&[3, 2, 1]);
        assert_eq!(p.eval(tf(5)), tf(38));
    }

    #[test]
    fn add_and_sub_are_inverse() {
        let a = poly(&[1, 2, 3]);
        let b = poly(&[4, 5]);
        let sum = a.add(&b);
        assert_eq!(sum.sub(&b), a);
    }

    #[test]
    fn mul_naive_matches_hand_computation() {
        // (x + 1)(x + 2) = x^2 + 3x + 2
        let a = poly(&[1, 1]);
        let b = poly(&[2, 1]);
        let product = a.mul_naive(&b);
        assert_eq!(product.coeffs(), &[tf(2), tf(3), tf(1)]);
    }

    #[test]
    fn mul_fft_matches_mul_naive() {
        let a = poly(&[1, 2, 3, 4, 5]);
        let b = poly(&[9, 8, 7]);
        assert_eq!(a.mul_naive(&b), a.mul_fft(&b));

        // also check with degrees that aren't already a tidy power of two
        let c = poly(&[1, 0, 4, 2, 9, 1, 6]);
        let d = poly(&[3, 3, 3, 3, 3]);
        assert_eq!(c.mul_naive(&d), c.mul_fft(&d));
    }

    #[test]
    fn mul_by_zero_is_zero() {
        let a = poly(&[1, 2, 3]);
        let z = Polynomial::zero();
        assert_eq!(a.mul_naive(&z), z);
        assert_eq!(a.mul_fft(&z), z);
    }
}
