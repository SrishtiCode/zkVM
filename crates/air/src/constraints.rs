use crate::air_for_isa::{selected_value, Row};
use field::Field;
use isa::{opcode_id, NUM_REGISTERS};

pub struct Check<F: Field> {
    pub name: &'static str,
    pub value: F,
}

impl<F: Field> Check<F> {
    fn new(name: &'static str, value: F) -> Self {
        Check { name, value }
    }

    pub fn holds(&self) -> bool {
        self.value.is_zero()
    }
}

pub fn transition_checks<F: Field>(cur: &Row<F>, next: &Row<F>) -> Vec<Check<F>> {
    let mut checks = Vec::new();

    let sel_sum = cur.sel.iter().fold(F::zero(), |acc, &s| acc + s);
    checks.push(Check::new("opcode_selector_sums_to_one", sel_sum - F::one()));
    for &s in cur.sel.iter() {
        checks.push(Check::new("opcode_selector_is_boolean", s * (s - F::one())));
    }

    push_one_hot_checks(&mut checks, "write", &cur.is_write_r);
    push_one_hot_checks(&mut checks, "read_a", &cur.is_read_a_r);
    push_one_hot_checks(&mut checks, "read_b", &cur.is_read_b_r);

    let read_a_val = selected_value(&cur.is_read_a_r, &cur.registers);
    let read_b_val = selected_value(&cur.is_read_b_r, &cur.registers);

    let write_value = cur.sel[opcode_id::LOAD_IMM] * cur.imm
        + cur.sel[opcode_id::LOAD] * cur.mem_read_value
        + cur.sel[opcode_id::ADD] * (read_a_val + read_b_val)
        + cur.sel[opcode_id::MUL] * (read_a_val * read_b_val);
    let writes_a_register = cur.sel[opcode_id::LOAD_IMM]
        + cur.sel[opcode_id::LOAD]
        + cur.sel[opcode_id::ADD]
        + cur.sel[opcode_id::MUL];

    for r in 0..NUM_REGISTERS {
        let expected_next =
            cur.registers[r] + writes_a_register * cur.is_write_r[r] * (write_value - cur.registers[r]);
        checks.push(Check::new("register_transition", next.registers[r] - expected_next));
    }

    checks.push(Check::new(
        "store_writes_the_read_register",
        cur.sel[opcode_id::STORE] * (cur.mem_write_value - read_a_val),
    ));

    checks.push(Check::new("is_zero_kills_nonzero_witness", read_a_val * cur.jnz_is_zero));
    checks.push(Check::new(
        "is_zero_inverse_consistency",
        read_a_val * cur.jnz_inv - (F::one() - cur.jnz_is_zero),
    ));

    let falls_through =
        F::one() - cur.sel[opcode_id::JMP] - cur.sel[opcode_id::JNZ] - cur.sel[opcode_id::HALT];
    let jnz_branch_taken = F::one() - cur.jnz_is_zero;
    let expected_pc = cur.sel[opcode_id::JMP] * cur.addr
        + cur.sel[opcode_id::JNZ] * (cur.jnz_is_zero * (cur.pc + F::one()) + jnz_branch_taken * cur.addr)
        + cur.sel[opcode_id::HALT] * cur.pc
        + falls_through * (cur.pc + F::one());
    checks.push(Check::new("pc_transition", next.pc - expected_pc));

    checks.push(Check::new(
        "halt_is_sticky",
        cur.sel[opcode_id::HALT] * (F::one() - next.sel[opcode_id::HALT]),
    ));

    checks
}

pub fn first_row_boundary_checks<F: Field>(row: &Row<F>) -> Vec<Check<F>> {
    let mut checks = Vec::new();
    checks.push(Check::new("initial_pc_is_zero", row.pc));
    for r in 0..NUM_REGISTERS {
        checks.push(Check::new("initial_register_is_zero", row.registers[r]));
    }
    checks
}

pub fn last_row_boundary_checks<F: Field>(row: &Row<F>) -> Vec<Check<F>> {
    vec![Check::new("final_row_is_halted", row.sel[opcode_id::HALT] - F::one())]
}

pub fn boundary_checks<F: Field>(rows: &[Row<F>]) -> Vec<Check<F>> {
    assert!(!rows.is_empty(), "a trace needs at least one row to have boundary constraints");
    let mut checks = first_row_boundary_checks(&rows[0]);
    checks.extend(last_row_boundary_checks(rows.last().unwrap()));
    checks
}

fn push_one_hot_checks<F: Field>(checks: &mut Vec<Check<F>>, label: &'static str, bits: &[F; NUM_REGISTERS]) {
    let sum = bits.iter().fold(F::zero(), |acc, &b| acc + b);
    checks.push(Check::new(one_hot_sum_name(label), sum - F::one()));
    for &b in bits {
        checks.push(Check::new(one_hot_bool_name(label), b * (b - F::one())));
    }
}

fn one_hot_sum_name(label: &'static str) -> &'static str {
    match label {
        "write" => "write_index_sums_to_one",
        "read_a" => "read_a_index_sums_to_one",
        "read_b" => "read_b_index_sums_to_one",
        _ => "index_sums_to_one",
    }
}
fn one_hot_bool_name(label: &'static str) -> &'static str {
    match label {
        "write" => "write_index_is_boolean",
        "read_a" => "read_a_index_is_boolean",
        "read_b" => "read_b_index_is_boolean",
        _ => "index_is_boolean",
    }
}

pub fn check_trace<F: Field>(rows: &[Row<F>]) -> Result<(), String> {
    for check in boundary_checks(rows) {
        if !check.holds() {
            return Err(format!("boundary constraint `{}` failed", check.name));
        }
    }
    for i in 0..rows.len().saturating_sub(1) {
        for check in transition_checks(&rows[i], &rows[i + 1]) {
            if !check.holds() {
                return Err(format!("transition constraint `{}` failed at row {i}", check.name));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::air_for_isa::build_rows;
    use field::ToyField;
    use isa::{run_padded, Instruction};

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    fn add_program() -> Vec<Instruction<ToyField>> {
        vec![
            Instruction::LoadImm { reg: 0, imm: tf(2) },
            Instruction::LoadImm { reg: 1, imm: tf(3) },
            Instruction::Add { dst: 2, a: 0, b: 1 },
            Instruction::Halt,
        ]
    }

        fn countdown_loop_program() -> Vec<Instruction<ToyField>> {
        use Instruction::*;
        vec![
            /* 0 */ LoadImm { reg: 0, imm: tf(5) },
            /* 1 */ LoadImm { reg: 1, imm: -ToyField::one() },
            /* 2: loop */ Jnz { reg: 0, target: 4 },
            /* 3 */ Jmp { target: 6 },
            /* 4 */ Add { dst: 0, a: 0, b: 1 },
            /* 5 */ Jmp { target: 2 },
            /* 6 */ Halt,
        ]
    }

    #[test]
    fn honest_trace_satisfies_every_constraint() {
        let trace = run_padded(&add_program(), vec![], 8);
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_ok());
    }

    #[test]
    fn honest_loop_program_satisfies_every_constraint() {
        let trace = run_padded(&countdown_loop_program(), vec![], 32);
        assert_eq!(trace.final_registers[0], ToyField::zero());
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_ok());
    }

    #[test]
    fn honest_load_store_trace_satisfies_every_constraint() {
        use Instruction::*;
        let program = vec![
            LoadImm { reg: 0, imm: tf(77) },
            Store { addr: 3, reg: 0 },
            Load { reg: 1, addr: 3 },
            Halt,
        ];
        let trace = run_padded(&program, vec![], 8);
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_ok());
    }

    #[test]
    fn tampered_register_value_breaks_a_constraint() {
        let trace = run_padded(&add_program(), vec![], 8);
        let mut rows = build_rows(&trace);
        rows[2].registers[2] += ToyField::one();
        assert!(check_trace(&rows).is_err());
    }

    #[test]
    fn tampered_opcode_selector_breaks_a_constraint() {
        let trace = run_padded(&add_program(), vec![], 8);
        let mut rows = build_rows(&trace);
        rows[0].sel[opcode_id::MUL] = ToyField::one();
        assert!(check_trace(&rows).is_err());
    }

    #[test]
    fn tampered_pc_breaks_a_constraint() {
        let trace = run_padded(&add_program(), vec![], 8);
        let mut rows = build_rows(&trace);
        rows[1].pc += ToyField::one();
        assert!(check_trace(&rows).is_err());
    }

    #[test]
    fn tampered_memory_write_breaks_a_constraint() {
        use Instruction::*;
        let program = vec![LoadImm { reg: 0, imm: tf(9) }, Store { addr: 1, reg: 0 }, Halt];
        let trace = run_padded(&program, vec![], 8);
        let mut rows = build_rows(&trace);
        rows[1].mem_write_value += ToyField::one();
        assert!(check_trace(&rows).is_err());
    }

    #[test]
    fn truncated_trace_without_halt_fails_boundary_check() {
        let trace = isa::run(&add_program(), vec![], 3); 
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_err());
    }

    #[test]
    fn tampered_write_index_one_hot_breaks_a_constraint() {
        let trace = run_padded(&add_program(), vec![], 8);
        let mut rows = build_rows(&trace);
        rows[0].is_write_r[1] = ToyField::one();
        assert!(check_trace(&rows).is_err());
    }

    #[test]
    fn tampered_read_index_one_hot_redirects_a_read_and_is_caught() {
        let trace = run_padded(&add_program(), vec![], 8);
        let mut rows = build_rows(&trace);
        assert!(check_trace(&rows).is_err());
    }

    #[test]
    fn retargeting_write_index_to_a_different_valid_one_hot_breaks_the_result() {
        let trace = run_padded(&add_program(), vec![], 8);
        let mut rows = build_rows(&trace);
        rows[0].is_write_r = [ToyField::zero(); isa::NUM_REGISTERS];
        rows[0].is_write_r[1] = ToyField::one();
        assert!(check_trace(&rows).is_err());
    }

    #[test]
    fn extra_padding_rows_still_verify() {
        let trace = run_padded(&add_program(), vec![], 16);
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_ok());
    }
}
