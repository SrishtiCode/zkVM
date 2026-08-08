pub mod air_for_isa;
pub mod constraints;

pub use air_for_isa::{build_rows, Row, NUM_COLUMNS};
pub use constraints::{
    boundary_checks, check_trace, first_row_boundary_checks, last_row_boundary_checks, transition_checks, Check,
};

#[cfg(test)]
mod tests {
    use super::*;
    use field::{Field, ToyField};
    use isa::{run_padded, Instruction};

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    fn fibonacci_program(n: u64) -> Vec<Instruction<ToyField>> {
        use Instruction::*;
        vec![
            /*  0 */ LoadImm { reg: 0, imm: tf(0) },
            /*  1 */ LoadImm { reg: 1, imm: tf(1) },
            /*  2 */ LoadImm { reg: 2, imm: tf(n) },
            /*  3 */ LoadImm { reg: 4, imm: -ToyField::one() },
            /*  4 */ LoadImm { reg: 5, imm: tf(0) },
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
    fn fibonacci_trace_satisfies_the_whole_air() {
        let trace = run_padded(&fibonacci_program(10), vec![], 128);
        assert_eq!(trace.final_registers[0], tf(55));
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_ok());
    }

    #[test]
    fn squares_trace_satisfies_the_whole_air() {
        use Instruction::*;
        let mut program = Vec::new();
        for i in 1..=8u64 {
            program.push(LoadImm { reg: 0, imm: tf(i) });
            program.push(Mul { dst: 1, a: 0, b: 0 });
            program.push(Store { addr: (i - 1) as usize, reg: 1 });
        }
        program.push(Halt);

        let trace = run_padded(&program, vec![], 64);
        assert_eq!(trace.final_memory[3], tf(16)); // 4^2
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_ok());
    }

    #[test]
    fn power_trace_satisfies_the_whole_air() {
        use Instruction::*;
        let program = vec![
            /* 0 */ LoadImm { reg: 0, imm: tf(1) },
            /* 1 */ LoadImm { reg: 1, imm: tf(3) },
            /* 2 */ LoadImm { reg: 2, imm: tf(6) },
            /* 3 */ LoadImm { reg: 3, imm: -ToyField::one() },
            /* 4 */ Jnz { reg: 2, target: 6 },
            /* 5 */ Jmp { target: 9 },
            /* 6 */ Mul { dst: 0, a: 0, b: 1 },
            /* 7 */ Add { dst: 2, a: 2, b: 3 },
            /* 8 */ Jmp { target: 4 },
            /* 9 */ Halt,
        ];
        let trace = run_padded(&program, vec![], 64);
        // 3^6 mod 97 = 729 mod 97 = 50
        assert_eq!(trace.final_registers[0], tf(50));
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_ok());
    }

    #[test]
    fn works_over_goldilocks_too() {
        use field::Goldilocks;
        use Instruction::*;
        let program: Vec<Instruction<Goldilocks>> = vec![
            LoadImm { reg: 0, imm: Goldilocks::from_u64(1000) },
            LoadImm { reg: 1, imm: Goldilocks::from_u64(2000) },
            Add { dst: 2, a: 0, b: 1 },
            Halt,
        ];
        let trace = run_padded(&program, vec![], 8);
        assert_eq!(trace.final_registers[2], Goldilocks::from_u64(3000));
        let rows = build_rows(&trace);
        assert!(check_trace(&rows).is_ok());
    }
}
