use field::Field;
pub const NUM_REGISTERS: usize = 6;
pub const MEMORY_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction<F: Field>{
    LoadImm{reg: usize, imm: F},
    Load{reg:usize, addr: usize},
    Store{addr: usize, reg: usize},
    Add{dst: usize, a: usize, b: usize},
    Mul{dst: usize, a: usize, b: usize},
    Jmp{target: usize},
    Jnz{reg: usize, target: usize},
    Halt,
}

pub mod opcode_id {
    pub const LOAD_IMM: usize = 0;
    pub const LOAD: usize = 1;
    pub const STORE: usize = 2;
    pub const ADD: usize = 3;
    pub const MUL: usize = 4;
    pub const JMP: usize = 5;
    pub const JNZ: usize = 6;
    pub const HALT: usize = 7;
}

pub const NUM_OPCODES: usize = 8;

impl<F: Field> Instruction<F> {

    pub fn opcode_index(&self) -> usize {
        match self {
            Instruction::LoadImm { .. } => 0,
            Instruction::Load { .. } => 1,
            Instruction::Store { .. } => 2,
            Instruction::Add { .. } => 3,
            Instruction::Mul { .. } => 4,
            Instruction::Jmp { .. } => 5,
            Instruction::Jnz { .. } => 6,
            Instruction::Halt => 7,
        }
    }

    pub fn mnemonic(&self) -> &'static str {
        match self {
            Instruction::LoadImm { .. } => "loadimm",
            Instruction::Load { .. } => "load",
            Instruction::Store { .. } => "store",
            Instruction::Add { .. } => "add",
            Instruction::Mul { .. } => "mul",
            Instruction::Jmp { .. } => "jmp",
            Instruction::Jnz { .. } => "jnz",
            Instruction::Halt => "halt",
        }
    }
}

impl<F: Field> std::fmt::Display for Instruction<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::LoadImm { reg, imm } => write!(f, "loadimm r{reg}, {}", imm.to_canonical_u64()),
            Instruction::Load { reg, addr } => write!(f, "load    r{reg}, [{addr}]"),
            Instruction::Store { addr, reg } => write!(f, "store   [{addr}], r{reg}"),
            Instruction::Add { dst, a, b } => write!(f, "add     r{dst}, r{a}, r{b}"),
            Instruction::Mul { dst, a, b } => write!(f, "mul     r{dst}, r{a}, r{b}"),
            Instruction::Jmp { target } => write!(f, "jmp     {target}"),
            Instruction::Jnz { reg, target } => write!(f, "jnz     r{reg}, {target}"),
            Instruction::Halt => write!(f, "halt"),
        }
    }
}

pub fn validate_instruction<F: Field>(instr: &Instruction<F>) -> Result<(), String> {
    let check_reg = |r: usize, name: &str| -> Result<(), String> {
        if r >= NUM_REGISTERS {
            Err(format!("register index {r} (in {name}) out of range 0..{NUM_REGISTERS}"))
        } else {
            Ok(())
        }
    };
    let check_addr = |a: usize, name: &str| -> Result<(), String> {
        if a >= MEMORY_SIZE {
            Err(format!("memory address {a} (in {name}) out of range 0..{MEMORY_SIZE}"))
        } else {
            Ok(())
        }
    };
    match *instr {
        Instruction::LoadImm { reg, .. } => check_reg(reg, "reg"),
        Instruction::Load { reg, addr } => check_reg(reg, "reg").and_then(|_| check_addr(addr, "addr")),
        Instruction::Store { addr, reg } => check_addr(addr, "addr").and_then(|_| check_reg(reg, "reg")),
        Instruction::Add { dst, a, b } => {
            check_reg(dst, "dst").and_then(|_| check_reg(a, "a")).and_then(|_| check_reg(b, "b"))
        }
        Instruction::Mul { dst, a, b } => {
            check_reg(dst, "dst").and_then(|_| check_reg(a, "a")).and_then(|_| check_reg(b, "b"))
        }
        Instruction::Jmp { .. } => Ok(()),
        Instruction::Jnz { reg, .. } => check_reg(reg, "reg"),
        Instruction::Halt => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;

    #[test]
    fn opcode_indices_are_distinct() {
        let instrs: Vec<Instruction<ToyField>> = vec![
            Instruction::LoadImm { reg: 0, imm: ToyField::from_u64(0) },
            Instruction::Load { reg: 0, addr: 0 },
            Instruction::Store { addr: 0, reg: 0 },
            Instruction::Add { dst: 0, a: 0, b: 0 },
            Instruction::Mul { dst: 0, a: 0, b: 0 },
            Instruction::Jmp { target: 0 },
            Instruction::Jnz { reg: 0, target: 0 },
            Instruction::Halt,
        ];
        let mut indices: Vec<usize> = instrs.iter().map(|i| i.opcode_index()).collect();
        indices.sort();
        indices.dedup();
        assert_eq!(indices.len(), NUM_OPCODES);
    }

    #[test]
    fn validate_catches_out_of_range_register() {
        let bad: Instruction<ToyField> = Instruction::Add { dst: NUM_REGISTERS, a: 0, b: 0 };
        assert!(validate_instruction(&bad).is_err());
        let good: Instruction<ToyField> = Instruction::Add { dst: 0, a: 1, b: 2 };
        assert!(validate_instruction(&good).is_ok());
    }

    #[test]
    fn validate_catches_out_of_range_memory_address() {
        let bad: Instruction<ToyField> = Instruction::Load { reg: 0, addr: MEMORY_SIZE };
        assert!(validate_instruction(&bad).is_err());
    }

    #[test]
    fn display_matches_mnemonic() {
        let i: Instruction<ToyField> = Instruction::Add { dst: 1, a: 2, b: 3 };
        assert_eq!(i.mnemonic(), "add");
        assert_eq!(format!("{i}"), "add     r1, r2, r3");
    }

    #[test]
    fn opcode_id_constants_match_opcode_index() {
        use crate::opcode_id;
        assert_eq!(
            Instruction::<ToyField>::LoadImm { reg: 0, imm: ToyField::from_u64(0) }.opcode_index(),
            opcode_id::LOAD_IMM
        );
        assert_eq!(Instruction::<ToyField>::Load { reg: 0, addr: 0 }.opcode_index(), opcode_id::LOAD);
        assert_eq!(Instruction::<ToyField>::Store { addr: 0, reg: 0 }.opcode_index(), opcode_id::STORE);
        assert_eq!(Instruction::<ToyField>::Add { dst: 0, a: 0, b: 0 }.opcode_index(), opcode_id::ADD);
        assert_eq!(Instruction::<ToyField>::Mul { dst: 0, a: 0, b: 0 }.opcode_index(), opcode_id::MUL);
        assert_eq!(Instruction::<ToyField>::Jmp { target: 0 }.opcode_index(), opcode_id::JMP);
        assert_eq!(Instruction::<ToyField>::Jnz { reg: 0, target: 0 }.opcode_index(), opcode_id::JNZ);
        assert_eq!(Instruction::<ToyField>::Halt.opcode_index(), opcode_id::HALT);
    }
}
