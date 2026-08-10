use field::Field;
use isa::{ExecutionTrace, Instruction, MEMORY_SIZE, NUM_OPCODES, NUM_REGISTERS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row<F:Field> {
    pub pc: F, 
    pub registers: [F; NUM_REGISTERS],
    pub sel: [F; NUM_OPCODES],
    pub is_write_r: [F; NUM_REGISTERS],
    pub is_read_a_r: [F; NUM_REGISTERS],
    pub is_read_b_r: [F; NUM_REGISTERS],
    pub addr: F,
    pub imm: F,
    pub mem_read_value: F,
    pub mem_write_value: F,
    pub jnz_is_zero: F,
    pub jnz_inv: F,        
}

pub const NUM_COLUMNS: usize = 1
    + NUM_REGISTERS
    + NUM_OPCODES
    + NUM_REGISTERS
    + NUM_REGISTERS
    + NUM_REGISTERS
    + 1
    + 1
    + 1
    + 1
    + 1
    + 1;

impl<F: Field> Row<F>{
    pub fn to_columns(&self) -> Vec<F>{
        let mut v = Vec::with_capacity(NUM_COLUMNS);
        v.push(self.pc);
        v.extend_from_slice(&self.registers);
        v.extend_from_slice(&self.sel);
        v.extend_from_slice(&self.is_write_r);
        v.extend_from_slice(&self.is_read_a_r);
        v.extend_from_slice(&self.is_read_b_r);
        v.push(self.addr);
        v.push(self.imm);
        v.push(self.mem_read_value);
        v.push(self.mem_write_value);
        v.push(self.jnz_is_zero);
        v.push(self.jnz_inv);
        debug_assert_eq!(v.len(), NUM_COLUMNS);
        v
    }

    pub fn from_columns(cols: &[F]) -> Self {
        assert_eq!(cols.len(), NUM_COLUMNS, "expected exactly NUM_COLUMNS values");
        let mut i = 0;
        let mut take = |n: usize| -> &[F] {
            let s = &cols[i..i + n];
            i += n;
            s
        };
        let pc = take(1)[0];
        let mut registers = [F::zero(); NUM_REGISTERS];
        registers.copy_from_slice(take(NUM_REGISTERS));
        let mut sel = [F::zero(); NUM_OPCODES];
        sel.copy_from_slice(take(NUM_OPCODES));
        let mut is_write_r = [F::zero(); NUM_REGISTERS];
        is_write_r.copy_from_slice(take(NUM_REGISTERS));
        let mut is_read_a_r = [F::zero(); NUM_REGISTERS];
        is_read_a_r.copy_from_slice(take(NUM_REGISTERS));
        let mut is_read_b_r = [F::zero(); NUM_REGISTERS];
        is_read_b_r.copy_from_slice(take(NUM_REGISTERS));
        let addr = take(1)[0];
        let imm = take(1)[0];
        let mem_read_value = take(1)[0];
        let mem_write_value = take(1)[0];
        let jnz_is_zero = take(1)[0];
        let jnz_inv = take(1)[0];
        Row {
            pc,
            registers,
            sel,
            is_write_r,
            is_read_a_r,
            is_read_b_r,
            addr,
            imm,
            mem_read_value,
            mem_write_value,
            jnz_is_zero,
            jnz_inv,
        }
    }
}     

fn one_hot<F: Field>(index: usize) -> [F; NUM_REGISTERS] {
    let mut v = [F::zero(); NUM_REGISTERS];
    v[index] = F::one();
    v
}

pub fn build_rows<F: Field>(trace: &ExecutionTrace<F>) -> Vec<Row<F>> {
    trace.rows.iter().map(build_row).collect()
}

fn build_row<F: Field>(trace_row: &isa::TraceRow<F>) -> Row<F> {
    let registers = trace_row.registers;
    let mut sel = [F::zero(); NUM_OPCODES];
    sel[trace_row.instruction.opcode_index()] = F::one();

    let mut is_write_r = one_hot(0);
    let mut is_read_a_r = one_hot(0);
    let mut is_read_b_r = one_hot(0);
    let mut addr = F::zero();
    let mut imm = F::zero();

    match trace_row.instruction {
        Instruction::LoadImm { reg, imm: value } => {
            is_write_r = one_hot(reg);
            imm = value;
        }
        Instruction::Load { reg, addr: a } => {
            is_write_r = one_hot(reg);
            addr = F::from_u64(a as u64);
        }
        Instruction::Store { addr: a, reg } => {
            is_read_a_r = one_hot(reg);
            addr = F::from_u64(a as u64);
        }
        Instruction::Add { dst, a, b } | Instruction::Mul { dst, a, b } => {
            is_write_r = one_hot(dst);
            is_read_a_r = one_hot(a);
            is_read_b_r = one_hot(b);
        }
        Instruction::Jmp { target } => {
            addr = F::from_u64(target as u64);
        }
        Instruction::Jnz { reg, target } => {
            is_read_a_r = one_hot(reg);
            addr = F::from_u64(target as u64);
        }
        Instruction::Halt => {}
    }

    let mem_read_value = trace_row.memory_access.read.map(|(_, v)| v).unwrap_or(F::zero());
    let mem_write_value = trace_row.memory_access.write.map(|(_, v)| v).unwrap_or(F::zero());
    let read_a_val = selected_value(&is_read_a_r, &registers);
    let (jnz_is_zero, jnz_inv) = if read_a_val.is_zero() {
        (F::one(), F::zero())
    } else {
        (F::zero(), read_a_val.inverse().expect("checked nonzero above"))
    };

    Row {
        pc: F::from_u64(trace_row.pc as u64),
        registers,
        sel,
        is_write_r,
        is_read_a_r,
        is_read_b_r,
        addr,
        imm,
        mem_read_value,
        mem_write_value,
        jnz_is_zero,
        jnz_inv,
    }
}

pub fn selected_value<F: Field>(one_hot: &[F; NUM_REGISTERS], values: &[F; NUM_REGISTERS]) -> F {
    let mut acc = F::zero();
    for k in 0..NUM_REGISTERS {
        acc += one_hot[k] * values[k];
    }
    acc
}

pub const MAX_MEMORY_ADDRESS: usize = MEMORY_SIZE - 1;

#[cfg(test)]
mod tests {
    use super::*;
    use field::ToyField;
    use isa::{opcode_id, run_padded};

    fn tf(x: u64) -> ToyField {
        ToyField::from_u64(x)
    }

    #[test]
    fn build_rows_produces_one_row_per_cycle() {
        let program = vec![Instruction::LoadImm { reg: 0, imm: tf(5) }, Instruction::Halt];
        let trace = run_padded(&program, vec![], 4);
        let rows = build_rows(&trace);
        assert_eq!(rows.len(), 4);
    }

    #[test]
    fn loadimm_row_has_expected_selectors() {
        let program = vec![Instruction::LoadImm { reg: 2, imm: tf(7) }, Instruction::Halt];
        let trace = run_padded(&program, vec![], 2);
        let rows = build_rows(&trace);
        assert_eq!(rows[0].sel[opcode_id::LOAD_IMM], ToyField::one());
        assert_eq!(rows[0].is_write_r[2], ToyField::one());
        assert_eq!(rows[0].imm, tf(7));
    }

    #[test]
    fn selected_value_picks_out_the_one_hot_register() {
        let registers = [tf(10), tf(20), tf(30), tf(40), tf(50), tf(60)];
        assert_eq!(selected_value(&one_hot(3), &registers), tf(40));
    }

    #[test]
    fn jnz_is_zero_gadget_matches_the_actual_register_value() {
        let program = vec![
            Instruction::LoadImm { reg: 0, imm: tf(0) },
            Instruction::Jnz { reg: 0, target: 3 },
            Instruction::Halt,
            Instruction::Halt,
        ];
        let trace = run_padded(&program, vec![], 4);
        let rows = build_rows(&trace);
        // row 1 is the Jnz on a zero register.
        assert_eq!(rows[1].jnz_is_zero, ToyField::one());
        assert_eq!(rows[1].jnz_inv, ToyField::zero());
    }

    #[test]
    fn to_columns_from_columns_round_trips() {
        let program = vec![
            Instruction::LoadImm { reg: 3, imm: tf(17) },
            Instruction::Store { addr: 5, reg: 3 },
            Instruction::Load { reg: 1, addr: 5 },
            Instruction::Halt,
        ];
        let trace = run_padded(&program, vec![], 8);
        let rows = build_rows(&trace);
        for row in &rows {
            let cols = row.to_columns();
            assert_eq!(cols.len(), NUM_COLUMNS);
            let round_tripped = Row::from_columns(&cols);
            assert_eq!(&round_tripped, row);
        }
    }
}