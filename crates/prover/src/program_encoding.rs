use field::Field;
use isa::{opcode_id, Instruction};

fn encode_instruction<F: Field>(instr: &Instruction<F>) -> [F; 6] {
    let z = F::zero();
    match *instr {
        Instruction::LoadImm { reg, imm } => {
            [F::from_u64(opcode_id::LOAD_IMM as u64), F::from_u64(reg as u64), z, z, z, imm]
        }
        Instruction::Load { reg, addr } => {
            [F::from_u64(opcode_id::LOAD as u64), F::from_u64(reg as u64), z, z, F::from_u64(addr as u64), z]
        }
        Instruction::Store { addr, reg } => {
            [F::from_u64(opcode_id::STORE as u64), z, z, F::from_u64(reg as u64), F::from_u64(addr as u64), z]
        }
        Instruction::Add { dst, a, b } => [
            F::from_u64(opcode_id::ADD as u64),
            F::from_u64(dst as u64),
            F::from_u64(a as u64),
            F::from_u64(b as u64),
            z,
            z,
        ],
        Instruction::Mul { dst, a, b } => [
            F::from_u64(opcode_id::MUL as u64),
            F::from_u64(dst as u64),
            F::from_u64(a as u64),
            F::from_u64(b as u64),
            z,
            z,
        ],
        Instruction::Jmp { target } => {
            [F::from_u64(opcode_id::JMP as u64), z, z, z, F::from_u64(target as u64), z]
        }
        Instruction::Jnz { reg, target } => {
            [F::from_u64(opcode_id::JNZ as u64), F::from_u64(reg as u64), z, z, F::from_u64(target as u64), z]
        }
        Instruction::Halt => [F::from_u64(opcode_id::HALT as u64), z, z, z, z, z],
    }
}

pub fn program_hash<F: Field>(hasher: &merkle::Poseidon<F>, program: &[Instruction<F>]) -> F {
    let mut flat = Vec::with_capacity(program.len() * 6);
    for instr in program {
        flat.extend_from_slice(&encode_instruction(instr));
    }
    hasher.hash_many(&flat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use field::{Field, ToyField};

    #[test]
    fn identical_programs_hash_the_same() {
        let hasher = merkle::Poseidon::new(5);
        let program: Vec<Instruction<ToyField>> =
            vec![Instruction::LoadImm { reg: 0, imm: ToyField::from_u64(5) }, Instruction::Halt];
        assert_eq!(program_hash(&hasher, &program), program_hash(&hasher, &program));
    }

    #[test]
    fn different_programs_hash_differently() {
        let hasher = merkle::Poseidon::new(5);
        let a: Vec<Instruction<ToyField>> =
            vec![Instruction::LoadImm { reg: 0, imm: ToyField::from_u64(5) }, Instruction::Halt];
        let b: Vec<Instruction<ToyField>> =
            vec![Instruction::LoadImm { reg: 0, imm: ToyField::from_u64(6) }, Instruction::Halt];
        assert_ne!(program_hash(&hasher, &a), program_hash(&hasher, &b));
    }
}