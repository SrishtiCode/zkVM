pub mod program_encoding;
pub mod prove;
pub mod statement;

pub use prove::prove;
pub use statement::{assert_row_shape, Proof, ProverParams, PublicClaim, QueryOpening, TraceRowOpening};

#[cfg(test)]
mod tests {
    use super::*;
    use field::{Field, Goldilocks};
    use isa::{Instruction, NUM_REGISTERS};
    use merkle::Poseidon;

    fn gf(x: u64) -> Goldilocks {
        Goldilocks::from_u64(x)
    }

    fn add_program() -> Vec<Instruction<Goldilocks>> {
        vec![
            Instruction::LoadImm { reg: 0, imm: gf(2) },
            Instruction::LoadImm { reg: 1, imm: gf(3) },
            Instruction::Add { dst: 2, a: 0, b: 1 },
            Instruction::Halt,
        ]
    }

    #[test]
    fn produces_a_proof_with_the_expected_shape() {
        let hasher: Poseidon<Goldilocks> = Poseidon::new(7);
        let mut claimed = [Goldilocks::zero(); NUM_REGISTERS];
        claimed[0] = gf(2);
        claimed[1] = gf(3);
        claimed[2] = gf(5);
        let claim = PublicClaim { claimed_final_registers: claimed };
        let params = ProverParams { blowup_factor: 16, fri_final_poly_num_coeffs: 1, num_queries: 8 };

        let proof = prove(&hasher, &add_program(), vec![], 8, params, claim);
        assert_eq!(proof.queries.len(), 8);
        assert_eq!(proof.trace_len, 8);
        for q in &proof.queries {
            assert_eq!(q.trace_here.values.len(), air::NUM_COLUMNS);
        }
    }
}
