//This cpu.rs is the part that actually executes the instructions defined in opcodes.rs.

/*
opcodes.rs                         cpu.rs

"What instructions exist?"        "What does each instruction DO?"

LoadImm  ────────────────────────► put value in register
Load     ────────────────────────► memory → register
Store    ────────────────────────► register → memory
Add      ────────────────────────► add two registers
Mul      ────────────────────────► multiply two registers
Jmp      ────────────────────────► change PC
Jnz      ────────────────────────► conditional change PC
Halt     ────────────────────────► stop
*/

use crate::opcodes::{Instruction, MEMORY_SIZE, NUM_REGISTERS};
use field::Field;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemoryAccess<F: Field>{
    pub read: Option<(usize, F)>,
    pub write: Option<(usize, F)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuState<F: Field>{
    pub pc: usize,
    pub registers: [F; NUM_REGISTERS],
    pub memory: Vec<F>,
    pub halted: bool,
}

impl<F: Field> CpuState<F>{
    pub fn new(mut initial_memory: Vec<F>) -> Self{
        assert!(
            initial_memory.len() <= MEMORY_SIZE,
            "initial memory ({} words) exceeds MEMORY_SIZE ({MEMORY_SIZE})",
            initial_memory.len()
        );
        initial_memory.resize(MEMORY_SIZE, F::zero());
        CpuState{ pc: 0, registers: [F::zero(); NUM_REGISTERS], memory: initial_memory, halted: false}
    }

    pub fn get_reg(&self, r:usize) -> F{
        self.registers[r]
    } 

    fn set_reg(&mut self, r: usize, value: F){
        self.registers[r] = value;
    } 

        pub fn step(&mut self, instr: &Instruction<F>, program_len: usize) -> MemoryAccess<F> {
        let mut access = MemoryAccess::default();
        match *instr {
            Instruction::LoadImm { reg, imm } => {
                self.set_reg(reg, imm);
                self.pc += 1;
            }
            Instruction::Load { reg, addr } => {
                let value = self.memory[addr];
                self.set_reg(reg, value);
                access.read = Some((addr, value));
                self.pc += 1;
            }
            Instruction::Store { addr, reg } => {
                let value = self.get_reg(reg);
                self.memory[addr] = value;
                access.write = Some((addr, value));
                self.pc += 1;
            }
            Instruction::Add { dst, a, b } => {
                let sum = self.get_reg(a) + self.get_reg(b);
                self.set_reg(dst, sum);
                self.pc += 1;
            }
            Instruction::Mul { dst, a, b } => {
                let product = self.get_reg(a) * self.get_reg(b);
                self.set_reg(dst, product);
                self.pc += 1;
            }
            Instruction::Jmp { target } => {
                assert!(target < program_len, "jmp target {target} out of program bounds");
                self.pc = target;
            }
            Instruction::Jnz { reg, target } => {
                if !self.get_reg(reg).is_zero() {
                    assert!(target < program_len, "jnz target {target} out of program bounds");
                    self.pc = target;
                } else {
                    self.pc += 1;
                }
            }
            Instruction::Halt => {
                self.halted = true;
            }
        }
        access
    }
} 

#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    #[test]
    fn loadimm_sets_register_and_advances_pc() {
        let mut cpu: CpuState<ToyField> = CpuState::new(vec![]);
        cpu.step(&Instruction::LoadImm { reg: 2, imm: tf(42) }, 1);
        assert_eq!(cpu.get_reg(2), tf(42));
        assert_eq!(cpu.pc, 1);
    }

    #[test]
    fn add_and_mul_use_field_arithmetic() {
        let mut cpu: CpuState<ToyField> = CpuState::new(vec![]);
        cpu.step(&Instruction::LoadImm { reg: 0, imm: tf(90) }, 4);
        cpu.step(&Instruction::LoadImm { reg: 1, imm: tf(10) }, 4);
        cpu.step(&Instruction::Add { dst: 2, a: 0, b: 1 }, 4);
        // 90 + 10 = 100 ≡ 3 (mod 97) — real field wraparound, no overflow bug.
        assert_eq!(cpu.get_reg(2), tf(3));
        cpu.step(&Instruction::Mul { dst: 3, a: 0, b: 1 }, 4);
        // 90 * 10 = 900 ≡ 27 (mod 97)
        assert_eq!(cpu.get_reg(3), tf(27));
    }

    #[test]
    fn load_and_store_round_trip_through_memory() {
        let mut cpu: CpuState<ToyField> = CpuState::new(vec![tf(0); 4]);
        cpu.step(&Instruction::LoadImm { reg: 0, imm: tf(55) }, 3);
        let access = cpu.step(&Instruction::Store { addr: 2, reg: 0 }, 3);
        assert_eq!(access.write, Some((2, tf(55))));
        assert_eq!(cpu.memory[2], tf(55));

        let access = cpu.step(&Instruction::Load { reg: 1, addr: 2 }, 3);
        assert_eq!(access.read, Some((2, tf(55))));
        assert_eq!(cpu.get_reg(1), tf(55));
    }

    #[test]
    fn jmp_sets_pc_directly() {
        let mut cpu: CpuState<ToyField> = CpuState::new(vec![]);
        cpu.step(&Instruction::Jmp { target: 3 }, 5);
        assert_eq!(cpu.pc, 3);
    }

    #[test]
    fn jnz_branches_on_nonzero_and_falls_through_on_zero() {
        let mut cpu: CpuState<ToyField> = CpuState::new(vec![]);
        cpu.step(&Instruction::LoadImm { reg: 0, imm: tf(0) }, 5);
        cpu.pc = 0;
        cpu.step(&Instruction::Jnz { reg: 0, target: 4 }, 5);
        assert_eq!(cpu.pc, 1, "zero register must fall through, not branch");

        cpu.step(&Instruction::LoadImm { reg: 0, imm: tf(1) }, 5);
        cpu.pc = 0;
        cpu.step(&Instruction::Jnz { reg: 0, target: 4 }, 5);
        assert_eq!(cpu.pc, 4, "nonzero register must branch");
    }

    #[test]
    fn halt_sets_flag_without_moving_pc() {
        let mut cpu: CpuState<ToyField> = CpuState::new(vec![]);
        cpu.pc = 7;
        cpu.step(&Instruction::Halt, 8);
        assert!(cpu.halted);
        assert_eq!(cpu.pc, 7);
        // Re-stepping is idempotent.
        cpu.step(&Instruction::Halt, 8);
        assert!(cpu.halted);
        assert_eq!(cpu.pc, 7);
    }

    #[test]
    #[should_panic(expected = "out of program bounds")]
    fn jmp_beyond_program_panics() {
        let mut cpu: CpuState<ToyField> = CpuState::new(vec![]);
        cpu.step(&Instruction::Jmp { target: 10 }, 3);
    }

    #[test]
    fn short_initial_memory_is_zero_padded() {
        let cpu: CpuState<ToyField> = CpuState::new(vec![tf(1), tf(2)]);
        assert_eq!(cpu.memory.len(), MEMORY_SIZE);
        assert_eq!(cpu.memory[0], tf(1));
        assert_eq!(cpu.memory[2], tf(0));
    }
}