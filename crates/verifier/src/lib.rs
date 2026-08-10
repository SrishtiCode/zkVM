use air::{Row, NUM_COLUMNS};
use field::Field;
use fri::FriRoundOpening;
use isa::{Instruction, NUM_REGISTERS};
use merkle::Poseidon;
use prover::program_encoding::program_hash;
use prover::{assert_row_shape, Proof, TraceRowOpening};
use transcript::Transcript;

fn verify_trace_opening<F: Field>(
    hasher: &Poseidon<F>,
    opening: &TraceRowOpening<F>,
    trace_root: F,
    expected_index: usize,
) -> bool {
    assert_row_shape(&opening.values);
    if opening.path.leaf_index != expected_index {
        return false;
    }
    let leaf_hash = hasher.hash_many(&opening.values);
    opening.path.verify(hasher, trace_root, leaf_hash)
}

#[allow(clippy::too_many_arguments)]
fn composition_value_at<F: Field>(
    row_values: &[F],
    next_row_values: &[F],
    x: F,
    trace_len: usize,
    last_trace_point: F,
    claimed_final_registers: &[F; NUM_REGISTERS],
    alpha_powers: &[F],
    num_transition: usize,
    num_first: usize,
) -> Option<F> {
    let cur_row = Row::from_columns(row_values);
    let next_row = Row::from_columns(next_row_values);

    let transition_vals = air::transition_checks(&cur_row, &next_row);
    let first_vals = air::first_row_boundary_checks(&cur_row);
    let mut last_vals = air::last_row_boundary_checks(&cur_row);
    for r in 0..NUM_REGISTERS {
        last_vals.push(air::Check {
            name: "claimed_final_register",
            value: cur_row.registers[r] - claimed_final_registers[r],
        });
    }

    let vanishing_h = x.pow(trace_len as u64) - F::one();
    let transition_zerofier = vanishing_h * (x - last_trace_point).inverse()?;
    let first_zerofier = x - F::one();
    let last_zerofier = x - last_trace_point;

    let mut acc_transition = F::zero();
    for (i, c) in transition_vals.iter().enumerate() {
        acc_transition += alpha_powers[i] * c.value;
    }
    let mut acc_first = F::zero();
    for (j, c) in first_vals.iter().enumerate() {
        acc_first += alpha_powers[num_transition + j] * c.value;
    }
    let mut acc_last = F::zero();
    for (m, c) in last_vals.iter().enumerate() {
        acc_last += alpha_powers[num_transition + num_first + m] * c.value;
    }

    Some(
        acc_transition * transition_zerofier.inverse()?
            + acc_first * first_zerofier.inverse()?
            + acc_last * last_zerofier.inverse()?,
    )
}

pub fn verify<F: Field>(hasher: &Poseidon<F>, program: &[Instruction<F>], proof: &Proof<F>) -> bool {
    let trace_len = proof.trace_len;
    if !trace_len.is_power_of_two() {
        return false;
    }
    let blowup = proof.params.blowup_factor;
    let big_n = trace_len * blowup;
    let half = big_n / 2;
    let offset = F::from_u64(3);

    let expected_program_hash = program_hash(hasher, program);
    if expected_program_hash != proof.program_hash {
        return false;
    }

    let mut transcript = Transcript::new(hasher, "zkvm-cpu-air-v1");
    transcript.absorb_u64(trace_len as u64);
    transcript.absorb_u64(blowup as u64);
    transcript.absorb(offset);
    transcript.absorb(proof.program_hash);
    for &v in &proof.claim.claimed_final_registers {
        transcript.absorb(v);
    }
    transcript.absorb(proof.trace_root);

    let alpha = transcript.squeeze_field();

    let probe = Row::from_columns(&vec![F::zero(); NUM_COLUMNS]);
    let num_transition = air::transition_checks(&probe, &probe).len();
    let num_first = air::first_row_boundary_checks(&probe).len();
    let num_last = air::last_row_boundary_checks(&probe).len() + NUM_REGISTERS;
    let total_checks = num_transition + num_first + num_last;
    let mut alpha_powers = Vec::with_capacity(total_checks);
    let mut p = F::one();
    for _ in 0..total_checks {
        alpha_powers.push(p);
        p *= alpha;
    }
    let betas = fri::replay_commit_phase(&proof.fri_commit, &mut transcript);

    let bound = half;
    if bound == 0 || proof.params.num_queries != proof.queries.len() {
        return false;
    }
    let indices: Vec<usize> = (0..proof.params.num_queries).map(|_| transcript.squeeze_index(bound)).collect();

    let log_n = trace_len.trailing_zeros();
    let trace_generator = F::root_of_unity(log_n);
    let last_trace_point = trace_generator.pow((trace_len - 1) as u64);
    let big_generator = F::root_of_unity(big_n.trailing_zeros());

    for (&idx0, query) in indices.iter().zip(&proof.queries) {
        let idx1 = idx0 + half;
        let next0 = (idx0 + blowup) % big_n;
        let next1 = (idx1 + blowup) % big_n;

        if !verify_trace_opening(hasher, &query.trace_here, proof.trace_root, idx0)
            || !verify_trace_opening(hasher, &query.trace_here_next, proof.trace_root, next0)
            || !verify_trace_opening(hasher, &query.trace_paired, proof.trace_root, idx1)
            || !verify_trace_opening(hasher, &query.trace_paired_next, proof.trace_root, next1)
        {
            return false;
        }

        if query.fri.is_empty() {
            return false;
        }
        let round0 = &query.fri[0];

        let x_here = fri::domain_point(offset, big_generator, idx0, 0);
        let x_paired = fri::domain_point(offset, big_generator, idx1, 0);

        let composition_here = match composition_value_at(
            &query.trace_here.values,
            &query.trace_here_next.values,
            x_here,
            trace_len,
            last_trace_point,
            &proof.claim.claimed_final_registers,
            &alpha_powers,
            num_transition,
            num_first,
        ) {
            Some(v) => v,
            None => return false,
        };
        let composition_paired = match composition_value_at(
            &query.trace_paired.values,
            &query.trace_paired_next.values,
            x_paired,
            trace_len,
            last_trace_point,
            &proof.claim.claimed_final_registers,
            &alpha_powers,
            num_transition,
            num_first,
        ) {
            Some(v) => v,
            None => return false,
        };

        if composition_here != round0.value_a || composition_paired != round0.value_b {
            return false;
        }
    }

    let fri_openings: Vec<Vec<FriRoundOpening<F>>> = proof.queries.iter().map(|q| q.fri.clone()).collect();
    fri::verify_queries(
        hasher,
        &proof.fri_commit,
        &betas,
        offset,
        big_n,
        proof.params.fri_final_layer_size(),
        proof.params.fri_final_poly_num_coeffs,
        &indices,
        &fri_openings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use field::Goldilocks;
    use isa::Instruction as I;
    use merkle::Poseidon;
    use prover::{prove, ProverParams, PublicClaim};

    fn gf(x: u64) -> Goldilocks {
        Goldilocks::from_u64(x)
    }

    fn add_program() -> Vec<I<Goldilocks>> {
        vec![
            I::LoadImm { reg: 0, imm: gf(2) },
            I::LoadImm { reg: 1, imm: gf(3) },
            I::Add { dst: 2, a: 0, b: 1 },
            I::Halt,
        ]
    }

    fn claim_for(values: &[(usize, u64)]) -> PublicClaim<Goldilocks> {
        let mut regs = [Goldilocks::zero(); NUM_REGISTERS];
        for &(i, v) in values {
            regs[i] = gf(v);
        }
        PublicClaim { claimed_final_registers: regs }
    }

    #[test]
    fn honest_proof_verifies() {
        let hasher: Poseidon<Goldilocks> = Poseidon::new(7);
        let claim = claim_for(&[(0, 2), (1, 3), (2, 5)]);
        let params = ProverParams { blowup_factor: 16, fri_final_poly_num_coeffs: 1, num_queries: 12 };
        let proof = prove(&hasher, &add_program(), vec![], 8, params, claim);
        assert!(verify(&hasher, &add_program(), &proof));
    }

    #[test]
    fn tampered_trace_opening_is_rejected() {
        let hasher: Poseidon<Goldilocks> = Poseidon::new(7);
        let claim = claim_for(&[(0, 2), (1, 3), (2, 5)]);
        let params = ProverParams { blowup_factor: 16, fri_final_poly_num_coeffs: 1, num_queries: 12 };
        let mut proof = prove(&hasher, &add_program(), vec![], 8, params, claim);
        proof.queries[0].trace_here.values[0] += Goldilocks::from_u64(1);
        assert!(!verify(&hasher, &add_program(), &proof));
    }

    #[test]
    fn wrong_claimed_output_is_rejected() {
        let hasher: Poseidon<Goldilocks> = Poseidon::new(7);
        let honest_claim = claim_for(&[(0, 2), (1, 3), (2, 5)]);
        let params = ProverParams { blowup_factor: 16, fri_final_poly_num_coeffs: 1, num_queries: 12 };
        let proof = prove(&hasher, &add_program(), vec![], 8, params, honest_claim);
        let mut wrong_proof = proof;
        wrong_proof.claim = claim_for(&[(0, 2), (1, 3), (2, 6)]); // 2+3 != 6
        assert!(!verify(&hasher, &add_program(), &wrong_proof));
    }

    #[test]
    fn program_hash_mismatch_is_rejected() {
        let hasher: Poseidon<Goldilocks> = Poseidon::new(7);
        let claim = claim_for(&[(0, 2), (1, 3), (2, 5)]);
        let params = ProverParams { blowup_factor: 16, fri_final_poly_num_coeffs: 1, num_queries: 12 };
        let proof = prove(&hasher, &add_program(), vec![], 8, params, claim);
        let different_program: Vec<I<Goldilocks>> =
        vec![I::LoadImm { reg: 0, imm: gf(99) }, I::Halt];
        assert!(!verify(&hasher, &different_program, &proof));
    }

    #[test]
    fn corrupted_fri_final_polynomial_is_rejected() {
        let hasher: Poseidon<Goldilocks> = Poseidon::new(7);
        let claim = claim_for(&[(0, 2), (1, 3), (2, 5)]);
        let params = ProverParams { blowup_factor: 16, fri_final_poly_num_coeffs: 1, num_queries: 12 };
        let mut proof = prove(&hasher, &add_program(), vec![], 8, params, claim);
        proof.fri_commit.final_poly_coeffs[0] += Goldilocks::from_u64(1);
        assert!(!verify(&hasher, &add_program(), &proof));
    }

    fn fibonacci_program(n: u64) -> Vec<I<Goldilocks>> {
        use I::*;
        vec![
            /*  0 */ LoadImm { reg: 0, imm: gf(0) },
            /*  1 */ LoadImm { reg: 1, imm: gf(1) },
            /*  2 */ LoadImm { reg: 2, imm: gf(n) },
            /*  3 */ LoadImm { reg: 4, imm: -Goldilocks::from_u64(1) },
            /*  4 */ LoadImm { reg: 5, imm: gf(0) },
            /*  5 */ Jnz { reg: 2, target: 7 },
            /*  6 */ Jmp { target: 12 },
            /*  7 */ Add { dst: 3, a: 0, b: 1 },
            /*  8 */ Add { dst: 0, a: 1, b: 5 },
            /*  9 */ Add { dst: 1, a: 3, b: 5 },
            /* 10 */ Add { dst: 2, a: 2, b: 4 },
            /* 11 */ Jmp { target: 5 },
            /* 12 */ Halt,
        ]
    }

    #[test]
    fn fibonacci_ten_proves_and_verifies_end_to_end() {
        let hasher: Poseidon<Goldilocks> = Poseidon::new(7);
        let program = fibonacci_program(10);
        let trace_len = 128;
        let params = ProverParams { blowup_factor: 16, fri_final_poly_num_coeffs: 1, num_queries: 24 };
        let honest_trace = isa::run_padded(&program, vec![], trace_len);
        let claim = PublicClaim { claimed_final_registers: honest_trace.final_registers };
        assert_eq!(claim.claimed_final_registers[0], gf(55), "fib(10) should be 55");

        let proof = prove(&hasher, &program, vec![], trace_len, params, claim);
        assert!(verify(&hasher, &program, &proof), "honest fib(10)=55 proof should verify");
        let mut wrong_registers = honest_trace.final_registers;
        wrong_registers[0] = gf(56);
        let wrong_claim = PublicClaim { claimed_final_registers: wrong_registers };
        let wrong_proof = prove(&hasher, &program, vec![], trace_len, params, wrong_claim);
        assert!(!verify(&hasher, &program, &wrong_proof), "fib(10)=56 is false and must not verify");
    }

}
