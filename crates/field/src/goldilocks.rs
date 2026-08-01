// Goldilocks - the finite field GF(2^64-2^32+1)
// p = 2^64 - 2^32 + 1 is chosen because :
// - it fits in a u64 so field elements are a single machine word,
// p - 1 = 2^32 * 3 * 5 * 17 * 257 * 65537 ,  giving a 2-adicity of 32 FFTs up to size 2^32 are supported.

use crate::Field;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub const  MODULUS: u64 = 0xFFFF_FFFF_0000_0001;

const ROOT_OF_UNITY_32: u64 =1_753_635_133_440_165_772;

#[derive(Copy, Clone, PartialEq, Eq, Default, Hash)]
pub struct Goldilocks(u64);

impl Goldilocks{
    pub fn new(value: u64) -> Self {
        if value >= MODULUS {
            Goldilocks(value - MODULUS)
        } else {
            Goldilocks(value)
        }   
    }

    pub fn value(&self) -> u64{
        self.0
    }
    
    fn mulmod(a: u64, b: u64) -> u64{
        ((a as u128 * b as u128) % MODULUS as u128) as u64     
    } 

    fn addmod(a: u64, b: u64) -> u64{
        ((a as u128 + b as u128) % MODULUS as u128) as u64 
    }     

    fn submod(a: u64, b: u64) -> u64{
        if a>=b{
            a-b
        }else{
            MODULUS - (b-a)  
        }  
    }    
}  

impl fmt::Debug for Goldilocks{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result{
        write!(f, "{}", self.0)
    }   
}   

impl fmt::Display for Goldilocks{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result{
        write!(f, "{}", self.0)
    }    
}   

impl Add for Goldilocks{
    type Output = Self;
    fn add(self, rhs: Self) -> Self{
        Goldilocks(Self::addmod(self.0, rhs.0))
    }      
}    

impl Sub for Goldilocks{
    type Output = Self;
    fn sub(self, rhs: Self) -> Self{
        Goldilocks(Self::submod(self.0, rhs.0))
    }       
} 

impl Mul for Goldilocks{
    type Output = Self;
    fn mul(self, rhs: Self) -> Self{
        Goldilocks(Self::mulmod(self.0, rhs.0))
    }      
}   

impl Neg for Goldilocks{
    type Output = Self;
    fn neg(self) -> Self{
        if self.0 == 0{
            self
        } else {
            Goldilocks(MODULUS - self.0)
        }
    }  
} 

impl AddAssign for Goldilocks{
    fn add_assign(&mut self, rhs: Self){
        *self = *self + rhs;     
    }  
} 

impl SubAssign for Goldilocks {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl MulAssign for Goldilocks {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Field for Goldilocks {
    fn zero() -> Self {
        Goldilocks(0)
    }

    fn one() -> Self {
        Goldilocks(1)
    }

    fn from_u64(x: u64) -> Self {
        Goldilocks::new(x)
    }

    fn modulus() -> u64 {
        MODULUS
    }

    fn inverse(&self) -> Option<Self>{
        if self.0 == {
            None
        } else {
            Some(self.pow(MODULUS - 2))
        }
    }   

    const TWO_ADICITY: u32 = 32;

    fn primitive_root_of_unity() -> Self{
        Goldilocks(ROOT_OF_UNITY_32)
    }
}    

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_laws;

    #[test]
    fn field_laws_hold() {
        field_laws::run_all::<Goldilocks>();
    }

    #[test]
    fn reduces_on_construction() {
        assert_eq!(Goldilocks::new(MODULUS).value(), 0);
        assert_eq!(Goldilocks::new(MODULUS + 5).value(), 5);
    }

    #[test]
    fn add_near_overflow_boundary() {
        let a = Goldilocks::new(MODULUS - 1);
        let b = Goldilocks::new(2);
        // (p - 1) + 2 = p + 1 ≡ 1 (mod p)
        assert_eq!((a + b).value(), 1);
    }

    #[test]
    fn root_of_unity_has_order_exactly_2_pow_32() {
        let root = Goldilocks::primitive_root_of_unity();
        assert_eq!(root.pow(1u64 << 32), Goldilocks::one());
        assert_ne!(root.pow(1u64 << 31), Goldilocks::one());
    }

    #[test]
    fn root_of_unity_domains_of_various_sizes() {
        for log_n in [0u32, 1, 2, 3, 8, 16, 32] {
            let n = 1u64 << log_n;
            let root = Goldilocks::root_of_unity(log_n);
            assert_eq!(root.pow(n), Goldilocks::one());
            if n > 1 {
                assert_ne!(root.pow(n / 2), Goldilocks::one());
            }
        }
    }

    #[test]
    fn inverse_matches_extended_euclid_reference() {
        // cross-check Fermat-based inverse against a from-scratch extended
        // Euclid implementation, for a handful of values.
        fn ext_euclid_inverse(a: u64, p: u64) -> u64 {
            let (mut old_r, mut r) = (a as i128, p as i128);
            let (mut old_s, mut s) = (1i128, 0i128);
            while r != 0 {
                let q = old_r / r;
                let tmp_r = old_r - q * r;
                old_r = r;
                r = tmp_r;
                let tmp_s = old_s - q * s;
                old_s = s;
                s = tmp_s;
            }
            (((old_s % p as i128) + p as i128) % p as i128) as u64
        }

        for x in [1u64, 2, 3, 12345, 999_999_999, MODULUS - 1] {
            let a = Goldilocks::new(x);
            let expected = ext_euclid_inverse(a.value(), MODULUS);
            assert_eq!(a.inverse().unwrap().value(), expected);
        }
    }
}    