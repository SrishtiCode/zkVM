use field::Field;
use merkle::poseidon::{RATE, T};
use merkle::Poseidon;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

pub struct Transcript<'h, F: Field>{
    hasher: &'h Poseidon<F>,
    state: [F; T],
    absorb_pos: usize,
    squeeze_pos: usize,
}

impl<'h, F:Field> Transcript<'h, F>{
    pub fn new(hasher: &'h Poseidon<F>, label: &str) -> Self{
        let mut t = Transcript { hasher, state: [F::zero(); T], absorb_pos: 0, squeeze_pos: RATE};
        t.absorb(F::from_u64(fnv1a(label.as_bytes())));
        t
    }

    fn permute(&mut self){
        self.state = self.hasher.permute(self.state);
        self.absorb_pos = 0;
        self.squeeze_pos = 0;
    }

    pub fn absorb(&mut self, value: F){
        if self.absorb_pos == RATE{
            self.permute();
        }
        self.state[self.absorb_pos] += value;
        self.absorb_pos += 1;
        self.squeeze_pos = RATE;
    }

    pub fn absorb_many(&mut self, values: &[F]){
        for &v in values {
            self.absorb(v);
        }
    }

    pub fn absorb_u64(&mut self, value: u64){
        self.absorb(F::from_u64(value));
    }

    pub fn squeeze_field(&mut self) -> F{
        if self.absorb_pos != 0 || self.squeeze_pos >= RATE{
            self.permute();
        }
        let out = self.state[self.squeeze_pos];
        self.squeeze_pos += 1;
        out
    }

    pub fn squeeze_index(&mut self, bound: usize) -> usize{
        (self.squeeze_field().to_canonical_u64() % bound as u64) as usize
    }
} 

impl<'h, F:Field> fri::ChallengeSource<F> for Transcript<'h, F>{
    fn absorb(&mut self, value: F){
        Transcript::absorb(self, value);
    }
    fn squeeze_field(&mut self) -> F {
        Transcript::squeeze_field(self)
    }
    fn squeeze_index(&mut self, bound: usize) -> usize{
        Transcript::squeeze_index(self, bound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use field::{Goldilocks, ToyField};

    fn hasher() -> Poseidon<ToyField> {
        Poseidon::new(5)
    }

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    #[test]
    fn is_deterministic_given_the_same_sequence_of_calls() {
        let h = hasher();
        let mut t1 = Transcript::new(&h, "test");
        let mut t2 = Transcript::new(&h, "test");
        t1.absorb(tf(10));
        t2.absorb(tf(10));
        assert_eq!(t1.squeeze_field(), t2.squeeze_field());
    }

    #[test]
    fn different_labels_diverge_immediately() {
        let h = hasher();
        let mut t1 = Transcript::new(&h, "protocol-a");
        let mut t2 = Transcript::new(&h, "protocol-b");
        assert_ne!(t1.squeeze_field(), t2.squeeze_field());
    }

    #[test]
    fn absorbing_different_data_changes_future_challenges() {
        let h = hasher();
        let mut t1 = Transcript::new(&h, "test");
        let mut t2 = Transcript::new(&h, "test");
        t1.absorb(tf(1));
        t2.absorb(tf(2));
        assert_ne!(t1.squeeze_field(), t2.squeeze_field());
    }

    #[test]
    fn repeated_squeezes_are_independent() {
        let h = hasher();
        let mut t = Transcript::new(&h, "test");
        t.absorb(tf(7));
        let a = t.squeeze_field();
        let b = t.squeeze_field();
        let c = t.squeeze_field();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn squeeze_field_stays_in_field_range() {
        let h = hasher();
        let mut t = Transcript::new(&h, "test");
        for _ in 0..20 {
            let f = t.squeeze_field();
            assert!(f.to_canonical_u64() < ToyField::modulus());
        }
    }

    #[test]
    fn absorbing_across_a_rate_boundary_still_matches_independently_built_transcript() {
        let h = hasher();
        let values: Vec<ToyField> = (1..=5).map(tf).collect();

        let mut t1 = Transcript::new(&h, "boundary-test");
        for &v in &values {
            t1.absorb(v);
        }

        let mut t2 = Transcript::new(&h, "boundary-test");
        t2.absorb_many(&values);

        assert_eq!(t1.squeeze_field(), t2.squeeze_field());
    }

    #[test]
    fn interleaved_absorb_and_squeeze_is_deterministic() {
        let h = hasher();
        let run = || {
            let mut t = Transcript::new(&h, "interleaved");
            t.absorb(tf(1));
            let a = t.squeeze_field();
            t.absorb(tf(2));
            t.absorb(tf(3));
            let b = t.squeeze_field();
            let c = t.squeeze_field();
            t.absorb(tf(4));
            let d = t.squeeze_field();
            (a, b, c, d)
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn squeeze_index_is_within_bound() {
        let h = hasher();
        let mut t = Transcript::new(&h, "index-test");
        for _ in 0..50 {
            let idx = t.squeeze_index(16);
            assert!(idx < 16);
        }
    }

    #[test]
    fn works_over_goldilocks_too() {
        let h: Poseidon<Goldilocks> = Poseidon::new(7);
        let mut t1 = Transcript::new(&h, "goldilocks-test");
        let mut t2 = Transcript::new(&h, "goldilocks-test");
        t1.absorb(Goldilocks::from_u64(123456789));
        t2.absorb(Goldilocks::from_u64(123456789));
        assert_eq!(t1.squeeze_field(), t2.squeeze_field());
    }

    #[test]
    fn absorb_u64_matches_absorbing_the_equivalent_field_element() {
        let h = hasher();
        let mut t1 = Transcript::new(&h, "test");
        let mut t2 = Transcript::new(&h, "test");
        t1.absorb_u64(42);
        t2.absorb(tf(42));
        assert_eq!(t1.squeeze_field(), t2.squeeze_field());
    }
}
