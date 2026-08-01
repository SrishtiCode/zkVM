// This file defines the mathematical interface for finite fields.
// It answers the question: "What must a field be able to do?"
//
// This crate provides the finite field arithmetic used by the rest of the project.
//
// We have two field implementations:
//
// 1. ToyField
//    - Arithmetic modulo 97.
//    - Small numbers make the math easy to understand and visualize.
//
// 2. Goldilocks
//    - A 64-bit STARK-friendly field.
//    - p = 2^64 - 2^32 + 1.
//
// What is 2-adicity?
//
// 2-adicity tells us how many times we can divide (p - 1) by 2
// before the result becomes odd.
//
// Example:
// ToyField: p = 97
//
// p - 1 = 96
// 96 / 2 = 48
// 48 / 2 = 24
// 24 / 2 = 12
// 12 / 2 = 6
//  6 / 2 = 3  <- odd
//
// So the 2-adicity of ToyField is 5.
//
// Why do we care?
// A high 2-adicity gives us large power-of-two roots-of-unity domains,
// which are needed for FFTs used later in the STARK pipeline.

pub mod goldilocks;
pub mod toyfield;

use std::fmt::Debug;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub use goldilocks::Goldilocks;
pub use toyfield::ToyField;

pub trait Field:
    Copy
    + Clone 
    + Debug
    + Default
    + PartialEq
    + Eq
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Neg<Output = Self>
    + AddAssign
    + SubAssign
    + MulAssign
    + 'static
{
    fn zero() -> Self;//Every type that implements the Field trait must provide a way to return the additive identity 0 of that field.
    fn one() -> Self;//Every type implementing Field must provide its multiplicative identity, 1.
    fn is_zero(&self) -> bool {//Is this field element equal to the zero element?
        *self == Self::zero()
    }
    fn from_u64(x: 64) -> Self;
    fn modulus() -> u64;
    fn inverse(&self) -> Option<Self>;
    fn pow(&self, exp: u64) -> Self{
        let mut base = *self;
        let mut exp = exp;
        let mut acc = Self::one();
        while exp > 0{
            if exp & 1 == 1 {
                acc *= base;
            }
            base *= base;
            exp >>= 1; 
        }
        acc
    }
    
    const TWO_ADICITY: u32;
    
    fn primitive_root_of_unity() -> Self;
    
    fn root_of_unity(log_n: u32) -> Self{
        assert!(
            log_n <= Self::TWO_ADICITY,
            "requested domain of size 2^{log_n} exceeds field's 2-adicity 2^{}",
            Self::TWO_ADICITY
        );
        let mut root = Self::primitive_root_of_unity();
        for _in 0..(Self::TWO_ADICITY - log_n){
            root *= root;
        } 
        root
    }
}

#[cfg(test)]
pub(crate) mod field_laws {
    use super::Field;

    pub fn check_additive_identity<F: Field>() {
        let a = F::from_u64(1234567);
        assert_eq!(a + F::zero(), a);
        assert_eq!(F::zero() + a, a);
    }

    pub fn check_multiplicative_identity<F: Field>() {
        let a = F::from_u64(7654321);
        assert_eq!(a * F::one(), a);
        assert_eq!(F::one() * a, a);
    }

    pub fn check_additive_inverse<F: Field>() {
        let a = F::from_u64(42);
        assert_eq!(a + (-a), F::zero());
    }

    pub fn check_multiplicative_inverse<F: Field>() {
        for x in [1u64, 2, 3, 5, 97, 12345] {
            let a = F::from_u64(x);
            if a.is_zero() {
                continue;
            }
            let inv = a.inverse().expect("nonzero element must be invertible");
            assert_eq!(a * inv, F::one());
        }
        assert!(F::zero().inverse().is_none());
    }

    pub fn check_distributivity<F: Field>() {
        let a = F::from_u64(3);
        let b = F::from_u64(11);
        let c = F::from_u64(19);
        assert_eq!(a * (b + c), a * b + a * c);
    }

    pub fn check_root_of_unity<F: Field>() {
        let log_n = 3u32.min(F::TWO_ADICITY);
        let n = 1u64 << log_n;
        let root = F::root_of_unity(log_n);
        // root^n == 1
        assert_eq!(root.pow(n), F::one());
        if n > 1 {
            assert_ne!(root.pow(n / 2), F::one());
        }
    }

    pub fn run_all<F: Field>() {
        check_additive_identity::<F>();
        check_multiplicative_identity::<F>();
        check_additive_inverse::<F>();
        check_multiplicative_inverse::<F>();
        check_distributivity::<F>();
        check_root_of_unity::<F>();
    }
}

