use air::{Check, Row, NUM_COLUMNS};
use field::Field;
use fri::FriProver;
use isa::{Instruction, NUM_REGISTERS};
use merkle::{MerkleTree, Poseidon};
use poly::interpolate::fft_interpolate;
use poly::lde::{coset_domain, low_degree_extend};
use transcript::Transcript;

use crate::program_encoding::program_hash;
use crate::statement::{Proof, ProverParams, PublicClaim, QueryOpening, TraceRowOpening};

pub fn prove<F: Field>(
    hasher: &Poseidon<F>,
    program: &[Instruction<F>],
    initial_memory: Vec<F>,
    trace_len: usize,
    params: ProverParams,
    claim: PublicClaim<F>,
) -> Proof<F> {
    assert!(trace_len.is_power_of_two(), "trace_len must be a power of two");
    let blowup = params.blowup_factor;
    let big_n = trace_len * blowup;

    let trace = isa::run_padded(program, initial_memory, trace_len);
    let rows = air::build_rows(&trace);

    let mut columns: Vec<Vec<F>> = vec![Vec::with_capacity(trace_len); NUM_COLUMNS];
    for row in &rows {
        for (c, v) in row.to_columns().into_iter().enumerate() {
            columns[c].push(v);
        }
    }

    let offset = F::from_u64(3);
    let lde_evals: Vec<Vec<F>> = columns
        .iter()
        .map(|col| {
             let mut coeffs = fft_interpolate(col).coeffs().to_vec();
            coeffs.resize(trace_len, F::zero());
            low_degree_extend(&coeffs, blowup, offset)
        })
        .collect();
    let lde_domain = coset_domain(big_n, offset);

    let lde_rows: Vec<Vec<F>> =
        (0..big_n).map(|k| (0..NUM_COLUMNS).map(|c| lde_evals[c][k]).collect()).collect();
    let trace_tree = MerkleTree::from_rows(hasher, &lde_rows);
    let trace_root = trace_tree.root();

    let prog_hash = program_hash(hasher, program);

    let mut transcript = Transcript::new(hasher, "zkvm-cpu-air-v1");
    transcript.absorb_u64(trace_len as u64);
    transcript.absorb_u64(blowup as u64);
    transcript.absorb(offset);
    transcript.absorb(prog_hash);
    for &v in &claim.claimed_final_registers {
        transcript.absorb(v);
    }
    transcript.absorb(trace_root);

    let alpha = transcript.squeeze_field();

    let num_transition = air::transition_checks(&rows[0], &rows[0]).len();
    let num_first = air::first_row_boundary_checks(&rows[0]).len();
    let num_last = air::last_row_boundary_checks(&rows[0]).len() + NUM_REGISTERS;
    let total_checks = num_transition + num_first + num_last;
    let mut alpha_powers = Vec::with_capacity(total_checks);
    let mut p = F::one();
    for _ in 0..total_checks {
        alpha_powers.push(p);
        p *= alpha;
    }

    let log_n = trace_len.trailing_zeros();
    let trace_generator = F::root_of_unity(log_n);
    let last_trace_point = trace_generator.pow((trace_len - 1) as u64);

    let composition_evals: Vec<F> = (0..big_n)
        .map(|k| {
            let x = lde_domain[k];
            let cur_cols: Vec<F> = (0..NUM_COLUMNS).map(|c| lde_evals[c][k]).collect();
            let next_cols: Vec<F> = (0..NUM_COLUMNS).map(|c| lde_evals[c][(k + blowup) % big_n]).collect();
            let cur_row = Row::from_columns(&cur_cols);
            let next_row = Row::from_columns(&next_cols);

            let transition_vals = air::transition_checks(&cur_row, &next_row);
            let first_vals = air::first_row_boundary_checks(&cur_row);
            let mut last_vals = air::last_row_boundary_checks(&cur_row);
            for r in 0..NUM_REGISTERS {
                last_vals.push(Check {
                    name: "claimed_final_register",
                    value: cur_row.registers[r] - claim.claimed_final_registers[r],
                });
            }

            let vanishing_h = x.pow(trace_len as u64) - F::one();
            let transition_zerofier = vanishing_h * (x - last_trace_point).inverse().expect("x off H");
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

            acc_transition * transition_zerofier.inverse().expect("nonzero off H")
                + acc_first * first_zerofier.inverse().expect("x != 1 on the LDE coset")
                + acc_last * last_zerofier.inverse().expect("x != last trace point on the LDE coset")
        })
        .collect();

    let fri_prover = FriProver::commit(
        hasher,
        composition_evals,
        offset,
        params.fri_final_layer_size(),
        params.fri_final_poly_num_coeffs,
        &mut transcript,
    );

    let bound = fri_prover.index_bound();
    let query_indices: Vec<usize> = (0..params.num_queries).map(|_| transcript.squeeze_index(bound)).collect();
    let fri_openings = fri_prover.open(&query_indices);

    let half = big_n / 2;
    let trace_open = |j: usize| -> TraceRowOpening<F> {
        let values: Vec<F> = (0..NUM_COLUMNS).map(|c| lde_evals[c][j]).collect();
        crate::statement::assert_row_shape(&values);
        TraceRowOpening { values, path: trace_tree.open(j) }
    };

    let queries: Vec<QueryOpening<F>> = query_indices
        .iter()
        .zip(fri_openings)
        .map(|(&idx0, fri_opening)| QueryOpening {
            trace_here: trace_open(idx0),
            trace_here_next: trace_open((idx0 + blowup) % big_n),
            trace_paired: trace_open(idx0 + half),
            trace_paired_next: trace_open((idx0 + half + blowup) % big_n),
            fri: fri_opening,
        })
        .collect();

    Proof {
        claim,
        params,
        trace_len,
        trace_root,
        program_hash: prog_hash,
        fri_commit: fri_prover.proof.clone(),
        queries,
    }
}

