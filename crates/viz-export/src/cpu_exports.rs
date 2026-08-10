use air::Row;
use field::{Field, ToyField};
use isa::{Instruction, NUM_REGISTERS};
use serde::Serialize;

#[derive(Serialize)]
pub struct InstructionJson {
    pub mnemonic: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct TraceRowJson {
    pub cycle: usize,
    pub pc: usize,
    pub registers: [u64; NUM_REGISTERS],
    pub instruction_index: usize, 
    pub mem_read: Option<(usize, u64)>,
    pub mem_write: Option<(usize, u64)>,
}

#[derive(Serialize)]
pub struct AirRowJson {
    pub cycle: usize,
    pub pc: u64,
    pub registers: [u64; NUM_REGISTERS],
    pub sel: Vec<u64>,
    pub opcode_name: String,
    pub is_write_r: Vec<u64>,
    pub is_read_a_r: Vec<u64>,
    pub is_read_b_r: Vec<u64>,
    pub addr: u64,
    pub imm: u64,
    pub mem_read_value: u64,
    pub mem_write_value: u64,
    pub jnz_is_zero: u64,
    pub jnz_inv: u64,
    pub transition_checks: Vec<(String, u64)>,
}

#[derive(Serialize)]
pub struct CpuExport {
    pub field_modulus: u64,
    pub program: Vec<InstructionJson>,
    pub trace: Vec<TraceRowJson>,
    pub air_rows: Vec<AirRowJson>,
    pub final_registers: [u64; NUM_REGISTERS],
    pub final_memory: Vec<u64>,
}

fn opcode_names() -> [&'static str; 8] {
    ["loadimm", "load", "store", "add", "mul", "jmp", "jnz", "halt"]
}

pub fn demo_program() -> Vec<Instruction<ToyField>> {
    use Instruction::*;
    let one = ToyField::from_u64(1);
    vec![
        /* 0 */ LoadImm { reg: 0, imm: one },                   // result = 1
        /* 1 */ LoadImm { reg: 1, imm: ToyField::from_u64(3) }, // base = 3
        /* 2 */ LoadImm { reg: 2, imm: ToyField::from_u64(4) }, // counter = 4
        /* 3 */ LoadImm { reg: 3, imm: -one },                  // -1 constant
        /* 4 */ Jnz { reg: 2, target: 6 },
        /* 5 */ Jmp { target: 9 },
        /* 6 */ Mul { dst: 0, a: 0, b: 1 },
        /* 7 */ Add { dst: 2, a: 2, b: 3 },
        /* 8 */ Jmp { target: 4 },
        /* 9 */ Halt,
    ]
}

pub fn export_cpu(trace_len: usize) -> CpuExport {
    let program = demo_program();
    let trace = isa::run_padded(&program, vec![], trace_len);
    let rows: Vec<Row<ToyField>> = air::build_rows(&trace);

    let program_json: Vec<InstructionJson> = program
        .iter()
        .map(|i| InstructionJson { mnemonic: i.mnemonic().to_string(), text: i.to_string() })
        .collect();

    let trace_json: Vec<TraceRowJson> = trace
        .rows
        .iter()
        .map(|r| TraceRowJson {
            cycle: r.cycle,
            pc: r.pc,
            registers: r.registers.map(|v| v.to_canonical_u64()),
            instruction_index: r.pc,
            mem_read: r.memory_access.read.map(|(a, v)| (a, v.to_canonical_u64())),
            mem_write: r.memory_access.write.map(|(a, v)| (a, v.to_canonical_u64())),
        })
        .collect();

    let names = opcode_names();
    let air_rows: Vec<AirRowJson> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let next = &rows[(i + 1).min(rows.len() - 1)];
            let checks = air::transition_checks(row, next);
            let opcode_idx = row.sel.iter().position(|&s| s == ToyField::one()).unwrap_or(0);
            AirRowJson {
                cycle: i,
                pc: row.pc.to_canonical_u64(),
                registers: row.registers.map(|v| v.to_canonical_u64()),
                sel: row.sel.iter().map(|v| v.to_canonical_u64()).collect(),
                opcode_name: names[opcode_idx].to_string(),
                is_write_r: row.is_write_r.iter().map(|v| v.to_canonical_u64()).collect(),
                is_read_a_r: row.is_read_a_r.iter().map(|v| v.to_canonical_u64()).collect(),
                is_read_b_r: row.is_read_b_r.iter().map(|v| v.to_canonical_u64()).collect(),
                addr: row.addr.to_canonical_u64(),
                imm: row.imm.to_canonical_u64(),
                mem_read_value: row.mem_read_value.to_canonical_u64(),
                mem_write_value: row.mem_write_value.to_canonical_u64(),
                jnz_is_zero: row.jnz_is_zero.to_canonical_u64(),
                jnz_inv: row.jnz_inv.to_canonical_u64(),
                transition_checks: checks
                    .into_iter()
                    .map(|c| (c.name.to_string(), c.value.to_canonical_u64()))
                    .collect(),
            }
        })
        .collect();

    CpuExport {
        field_modulus: ToyField::modulus(),
        program: program_json,
        trace: trace_json,
        air_rows,
        final_registers: trace.final_registers.map(|v| v.to_canonical_u64()),
        final_memory: trace.final_memory.iter().map(|v| v.to_canonical_u64()).collect(),
    }
}

