// Toyfield - the finite field GF(97)

// 97 is prime, and 97 - 1 = 96 = 2^5 * 3, so the field has a 2-adic subgroup
// of size 2^5 = 32.                         

use crate::Field;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

// The modulus of the toy field
pub const MODULUS: u64 = 97;
//GENERATOR^((p-1)/32) mod p = 5^3 mod 97 = 28
const ROOT_OF_UNITY_32: u64 = 28;

#[derive(Copy, Clone, PartialEq, Eq, Default, Hash)]
pub struct ToyField(u64);

impl ToyField {
    /// Constructs an element from any `u64`, reducing mod 97.
    pub fn new(value: u64) -> Self {
        ToyField(value % MODULUS)
    }

    /// The raw representative in `0..97`.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Debug for ToyField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for ToyField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for ToyField {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        ToyField((self.0 + rhs.0) % MODULUS)
    }
}

impl Sub for ToyField {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        // add MODULUS before subtracting to stay in unsigned arithmetic
        ToyField((self.0 + MODULUS - rhs.0) % MODULUS)
    }
}

impl Mul for ToyField {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        ToyField((self.0 * rhs.0) % MODULUS)
    }
}

impl Neg for ToyField {
    type Output = Self;
    fn neg(self) -> Self {
        if self.0 == 0 {
            self
        } else {
            ToyField(MODULUS - self.0)
        }
    }
}

impl AddAssign for ToyField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for ToyField {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl MulAssign for ToyField {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Field for ToyField {
    fn zero() -> Self {
        ToyField(0)
    }

    fn one() -> Self {
        ToyField(1)
    }

    fn from_u64(x: u64) -> Self {
        ToyField::new(x)
    }

    fn modulus() -> u64 {
        MODULUS
    }

    fn to_canonical_u64(&self) -> u64 {
        self.0
    }

    fn inverse(&self) -> Option<Self> {
        if self.0 == 0 {
            return None;
        }
        let (mut old_r, mut r) = (self.0 as i64, MODULUS as i64);
        let (mut old_s, mut s) = (1i64, 0i64);
        while r != 0 {
            let q = old_r / r;
            (old_r, r) = (r, old_r - q * r);
            (old_s, s) = (s, old_s - q * s);
        }
        let inv = ((old_s % MODULUS as i64) + MODULUS as i64) % MODULUS as i64;
        Some(ToyField(inv as u64))
    }

    const TWO_ADICITY: u32 = 5;

    fn primitive_root_of_unity() -> Self {
        ToyField(ROOT_OF_UNITY_32)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_laws;

    const GENERATOR: u64 = 5;

    #[test]
    fn field_laws_hold() {
        field_laws::run_all::<ToyField>();
    }

    #[test]
    fn reduces_on_construction() {
        assert_eq!(ToyField::new(97).value(), 0);
        assert_eq!(ToyField::new(98).value(), 1);
        assert_eq!(ToyField::new(0).value(), 0);
    }

    #[test]
    fn arithmetic_wraps_mod_97() {
        let a = ToyField::new(90);
        let b = ToyField::new(10);
        assert_eq!((a + b).value(), 3); // 100 mod 97
        assert_eq!((a * b).value(), 27); // 900 mod 97 = 900 - 9*97 = 27
        let c = ToyField::new(5);
        let d = ToyField::new(9);
        assert_eq!((c - d).value(), 93); // 5 - 9 = -4 -> 93 mod 97
    }

    #[test]
    fn generator_has_full_order() {
        // 5^96 == 1, and 5^(96/2), 5^(96/3) both != 1 (order exactly 96).
        let g = ToyField::new(GENERATOR);
        assert_eq!(g.pow(96), ToyField::one());
        assert_ne!(g.pow(48), ToyField::one());
        assert_ne!(g.pow(32), ToyField::one());
    }

    #[test]
    fn root_of_unity_domains_of_various_sizes() {
        for log_n in 0..=5u32 {
            let n = 1u64 << log_n;
            let root = ToyField::root_of_unity(log_n);
            assert_eq!(root.pow(n), ToyField::one(), "root^n != 1 for n=2^{log_n}");
            if n > 1 {
                assert_ne!(
                    root.pow(n / 2),
                    ToyField::one(),
                    "root not primitive for n=2^{log_n}"
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "exceeds field's 2-adicity")]
    fn root_of_unity_beyond_two_adicity_panics() {
        let _ = ToyField::root_of_unity(6);
    }
}
