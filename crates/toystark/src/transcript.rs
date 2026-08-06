use crate::hash::hash_pair;
use field::Field;

pub struct Transcript{
    state: u64,
} 

impl Transcript{
    pub fn new(domain_separator: &[u64]) -> Self{
        let mut state = 0x517c_c1b7_2722_0a95;
        for &v in domain_separator {
            state = hash_pair(state, v); 
        }   
        Transcript{state}
    }

    pub fn absorb(&mut self, value: u64) {
        self.state = hash_pair(self.state, value); 
    }

    pub fn absorb_field<F:Field>(&mut self, value:F){
        self.absorb(value.to_canonical_u64());
    }

    pub fn squeeze_u64(&mut self) -> u64{
        self.state = hash_pair(self.state, 0xC0FF_EE00_C0FF_EE00);
        self.state  
    }   

    pub fn squeeze_field<F: Field>(&mut self) -> F {
        F::from_u64(self.squeeze_u64())
    }

    pub fn squeeze_index(&mut self, bound: usize) -> usize {
        (self.squeeze_u64() % bound as u64) as usize
    }

} 


#[cfg(test)]
mod tests {
    use super::*;
    use field::{Field, ToyField};

    #[test]
    fn is_deterministic_given_the_same_transcript_of_calls() {
        let mut t1 = Transcript::new(&[1, 2, 3]);
        let mut t2 = Transcript::new(&[1, 2, 3]);
        t1.absorb(10);
        t2.absorb(10);
        assert_eq!(t1.squeeze_u64(), t2.squeeze_u64());
    }

    #[test]
    fn different_domain_separators_diverge() {
        let mut t1 = Transcript::new(&[1]);
        let mut t2 = Transcript::new(&[2]);
        assert_ne!(t1.squeeze_u64(), t2.squeeze_u64());
    }

    #[test]
    fn absorbing_different_data_changes_future_challenges() {
        let mut t1 = Transcript::new(&[]);
        let mut t2 = Transcript::new(&[]);
        t1.absorb(1);
        t2.absorb(2);
        assert_ne!(t1.squeeze_u64(), t2.squeeze_u64());
    }

    #[test]
    fn repeated_squeezes_are_independent() {
        let mut t = Transcript::new(&[7]);
        let a = t.squeeze_u64();
        let b = t.squeeze_u64();
        assert_ne!(a, b);
    }

    #[test]
    fn squeeze_field_stays_in_field() {
        let mut t = Transcript::new(&[42]);
        let f: ToyField = t.squeeze_field();
        assert!(f.to_canonical_u64() < ToyField::modulus());
    }
}
