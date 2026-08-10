use field::Field;

pub const T: usize = 3;
pub const RATE: usize = T - 1;
const FULL_ROUNDS: usize = 8;
const PARTIAL_ROUNDS: usize = 22;
const TOTAL_ROUNDS: usize = FULL_ROUNDS + PARTIAL_ROUNDS;
  
fn gcd(a: u64, b: u64) -> u64{
    if b == 0{
        a
    }else{
        gcd(b, a % b)
    }
}

fn splitmix64_next(state: &mut u64) -> u64{
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9); 
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

pub struct Poseidon<F: Field>{
    alpha: u64,
    round_constants: Vec<[F; T]>,
    mds: [[F; T]; T],
}

impl<F: Field> Poseidon<F>{
    pub fn new(alpha: u64) -> Self{
        assert!(
            gcd(alpha, F::modulus() - 1) == 1,
                "S-box exponent {alpha} is not coprime to p-1={}; x^{alpha} would not be a bijection",
                F::modulus() - 1 
            );

            let mut seed: u64 = 0x504F5345_49444F4E;       
            let mut round_constants = Vec::with_capacity(TOTAL_ROUNDS);
            for _ in 0..TOTAL_ROUNDS {
                let mut row = [F::zero(); T];
                for slot in row.iter_mut() {
                    *slot = F::from_u64(splitmix64_next(&mut seed));
                }
                round_constants.push(row);
            }

        let mut mds = [[F::zero(); T]; T];
        for i in 0..T {
            for j in 0..T {
                let x_i = F::from_u64(i as u64);
                let y_j = F::from_u64((T + j) as u64);
                mds[i][j] = (x_i + y_j).inverse().expect("x_i + y_j chosen nonzero by construction");
            }
        }

        Poseidon { alpha, round_constants, mds }
    }

        fn mds_multiply(&self, state: [F; T]) -> [F; T] {
        let mut out = [F::zero(); T];
        for (i, out_slot) in out.iter_mut().enumerate() {
            let mut acc = F::zero();
            for j in 0..T {
                acc += self.mds[i][j] * state[j];
            }
            *out_slot = acc;
        }
        out
    }

    pub fn permute(&self, mut state: [F; T]) -> [F; T] {
        let half_full = FULL_ROUNDS / 2;
        for (r, constants) in self.round_constants.iter().enumerate() {
            for i in 0..T {
                state[i] += constants[i];
            }
            let is_full_round = r < half_full || r >= half_full + PARTIAL_ROUNDS;
            if is_full_round {
                for s in state.iter_mut() {
                    *s = s.pow(self.alpha);
                }
            } else {
                state[0] = state[0].pow(self.alpha);
            }
            state = self.mds_multiply(state);
        }
        state
    }

    pub fn hash_many(&self, inputs: &[F]) -> F {
        let mut state = [F::zero(); T];
        if inputs.is_empty() {
            return self.permute(state)[0];
        }
        for chunk in inputs.chunks(RATE) {
            for (i, &v) in chunk.iter().enumerate() {
                state[i] += v;
            }
            state = self.permute(state);
        }
        state[0]
    }

    pub fn hash_two(&self, left: F, right: F) -> F {
        self.hash_many(&[left, right])
    }

}
      
#[cfg(test)]
mod tests {
    use super::*;
    use field::{Goldilocks, ToyField};

    #[test]
    fn same_input_hashes_the_same_every_time() {
        let p: Poseidon<ToyField> = Poseidon::new(5);
        let a = p.hash_two(ToyField::from_u64(3), ToyField::from_u64(4));
        let b = p.hash_two(ToyField::from_u64(3), ToyField::from_u64(4));
        assert_eq!(a, b);
    }

    #[test]
    fn different_inputs_hash_differently() {
        let p: Poseidon<ToyField> = Poseidon::new(5);
        let a = p.hash_two(ToyField::from_u64(3), ToyField::from_u64(4));
        let b = p.hash_two(ToyField::from_u64(4), ToyField::from_u64(3));
        let c = p.hash_two(ToyField::from_u64(3), ToyField::from_u64(5));
        assert_ne!(a, b, "hash should not be symmetric in its two inputs");
        assert_ne!(a, c);
    }

    #[test]
    fn works_over_goldilocks_with_alpha_seven() {
        let p: Poseidon<Goldilocks> = Poseidon::new(7);
        let a = p.hash_two(Goldilocks::from_u64(100), Goldilocks::from_u64(200));
        let b = p.hash_two(Goldilocks::from_u64(100), Goldilocks::from_u64(200));
        assert_eq!(a, b);
        let c = p.hash_two(Goldilocks::from_u64(200), Goldilocks::from_u64(100));
        assert_ne!(a, c);
    }

    #[test]
    #[should_panic(expected = "not coprime")]
    fn rejects_a_non_coprime_alpha() {
        let _: Poseidon<Goldilocks> = Poseidon::new(5);
    }

    #[test]
    fn hash_many_handles_lengths_not_a_multiple_of_rate() {
        let p: Poseidon<ToyField> = Poseidon::new(5);
        let values: Vec<ToyField> = (1..=5).map(ToyField::from_u64).collect(); 
        let h = p.hash_many(&values);
        let h_again = p.hash_many(&values);
        assert_eq!(h, h_again);
    }

    #[test]
    fn hash_many_of_single_value_matches_hash_two_with_implicit_zero() {
        let p: Poseidon<ToyField> = Poseidon::new(5);
        let single = p.hash_many(&[ToyField::from_u64(9)]);
        let pair = p.hash_two(ToyField::from_u64(9), ToyField::from_u64(0));
        assert_eq!(single, pair, "hash_many([9]) absorbs the same padded chunk as hash_two(9, 0)");
    }

    #[test]
    fn mds_matrix_rows_are_distinct_and_nonzero() {
        let p: Poseidon<ToyField> = Poseidon::new(5);
        for row in &p.mds {
            assert!(row.iter().any(|&v| !v.is_zero()));
        }
        assert_ne!(p.mds[0], p.mds[1]);
        assert_ne!(p.mds[1], p.mds[2]);
    }

    #[test]
    fn permutation_is_deterministic_and_changes_the_state() {
        let p: Poseidon<ToyField> = Poseidon::new(5);
        let input = [ToyField::from_u64(1), ToyField::from_u64(2), ToyField::from_u64(3)];
        let out1 = p.permute(input);
        let out2 = p.permute(input);
        assert_eq!(out1, out2);
        assert_ne!(out1, input);
    }
}

