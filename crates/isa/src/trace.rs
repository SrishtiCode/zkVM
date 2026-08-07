use crate::cpu::{CpuState, MemoryAccess};
use crate::opcodes::{validate_instruction, Instruction, NUM_REGISTERS};
use field::Field;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceRow<F: Field> {
    pub cycle: usize,
    pub pc: usize,
    pub registers: [F; NUM_REGISTERS],
    pub instruction: Instruction<F>,
    pub memory_access: MemoryAccess<F>,
}

#[derive(Debug, Clone)]
pub struct ExecutionTrace<F: Field> {
    pub rows: Vec<TraceRow<F>>,
    pub final_registers: [F; NUM_REGISTERS],
    pub final_memory: Vec<F>,
    pub halted: bool,
}

impl<F: Field> ExecutionTrace<F> {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{:>5}  {:>3}  {:<28}  registers\n",
            "cycle", "pc", "instruction"
        ));
        for row in &self.rows {
            let regs: Vec<String> = row.registers.iter().map(|r| r.to_canonical_u64().to_string()).collect();
            out.push_str(&format!(
                "{:>5}  {:>3}  {:<28}  [{}]\n",
                row.cycle,
                row.pc,
                row.instruction.to_string(),
                regs.join(", ")
            ));
        }
        out
    }
}

fn run_impl<F: Field>(
    program: &[Instruction<F>],
    initial_memory: Vec<F>,
    max_cycles: usize,
    pad_after_halt: bool,
) -> ExecutionTrace<F> {
    for (i, instr) in program.iter().enumerate() {
        if let Err(reason) = validate_instruction(instr) {
            panic!("invalid instruction at program index {i}: {reason}");
        }
    }

    let mut cpu: CpuState<F> = CpuState::new(initial_memory);
    let mut rows = Vec::with_capacity(max_cycles);

    for cycle in 0..max_cycles {
        if cpu.halted && !pad_after_halt {
            break;
        }
        let pc = cpu.pc;
        assert!(
            pc < program.len(),
            "pc {pc} ran off the end of the program (length {}) at cycle {cycle}; \
             did you forget a Halt?",
            program.len()
        );
        let instruction = program[pc];
        let registers_before = cpu.registers;
        let memory_access = cpu.step(&instruction, program.len());
        rows.push(TraceRow { cycle, pc, registers: registers_before, instruction, memory_access });
    }

    if pad_after_halt {
        assert!(
            cpu.halted,
            "program did not halt within {max_cycles} cycles; increase trace_len or check for an \
             infinite loop"
        );
    }

    ExecutionTrace { rows, final_registers: cpu.registers, final_memory: cpu.memory, halted: cpu.halted }
}

pub fn run<F: Field>(program: &[Instruction<F>], initial_memory: Vec<F>, max_cycles: usize) -> ExecutionTrace<F> {
    run_impl(program, initial_memory, max_cycles, false)
}

pub fn run_padded<F: Field>(
    program: &[Instruction<F>],
    initial_memory: Vec<F>,
    trace_len: usize,
) -> ExecutionTrace<F> {
    run_impl(program, initial_memory, trace_len, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes::Instruction as I;
    use field::ToyField;

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    /// r0 <- 2; r1 <- 3; r2 <- r0 + r1; halt.
    fn tiny_program() -> Vec<I<ToyField>> {
        vec![
            I::LoadImm { reg: 0, imm: tf(2) },
            I::LoadImm { reg: 1, imm: tf(3) },
            I::Add { dst: 2, a: 0, b: 1 },
            I::Halt,
        ]
    }

    #[test]
    fn run_stops_as_soon_as_halted() {
        let trace = run(&tiny_program(), vec![], 100);
        assert_eq!(trace.rows.len(), 4);
        assert!(trace.halted);
        assert_eq!(trace.final_registers[2], tf(5));
    }

    #[test]
    fn rows_chain_correctly_state_before_matches_previous_after() {
        let trace = run(&tiny_program(), vec![], 100);
        assert_eq!(trace.rows[2].registers[0], tf(2));
        assert_eq!(trace.rows[2].registers[1], tf(3));
        assert_eq!(trace.rows[2].instruction, I::Add { dst: 2, a: 0, b: 1 });
    }

    #[test]
    fn run_padded_fills_remaining_cycles_with_idempotent_halts() {
        let trace = run_padded(&tiny_program(), vec![], 8);
        assert_eq!(trace.rows.len(), 8);
        assert!(trace.halted);
        for row in &trace.rows[3..] {
            assert_eq!(row.instruction, I::Halt);
            assert_eq!(row.pc, 3);
            assert_eq!(row.registers, trace.final_registers);
        }
    }

    #[test]
    #[should_panic(expected = "did not halt")]
    fn run_padded_panics_if_program_never_halts() {
        let infinite_loop: Vec<I<ToyField>> = vec![I::Jmp { target: 0 }];
        let _ = run_padded(&infinite_loop, vec![], 16);
    }

    #[test]
    #[should_panic(expected = "ran off the end")]
    fn run_panics_on_pc_overrun_without_halt() {
        let no_halt = vec![I::LoadImm { reg: 0, imm: tf(1) }];
        let _ = run(&no_halt, vec![], 5);
    }

    #[test]
    #[should_panic(expected = "invalid instruction")]
    fn run_rejects_malformed_program_up_front() {
        let bad: Vec<I<ToyField>> = vec![I::Add { dst: 99, a: 0, b: 0 }, I::Halt];
        let _ = run(&bad, vec![], 5);
    }

    #[test]
    fn memory_accesses_are_recorded_on_the_right_rows() {
        let program = vec![
            I::LoadImm { reg: 0, imm: tf(9) },
            I::Store { addr: 5, reg: 0 },
            I::Load { reg: 1, addr: 5 },
            I::Halt,
        ];
        let trace = run(&program, vec![], 10);
        assert_eq!(trace.rows[1].memory_access.write, Some((5, tf(9))));
        assert_eq!(trace.rows[2].memory_access.read, Some((5, tf(9))));
        assert_eq!(trace.rows[0].memory_access.write, None);
    }
}