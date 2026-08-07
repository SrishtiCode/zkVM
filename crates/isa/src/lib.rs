pub mod cpu;
pub mod opcodes;
pub mod trace;

pub use cpu::CpuState;
pub use opcodes::{opcode_id, Instruction, MEMORY_SIZE, NUM_OPCODES, NUM_REGISTERS};
pub use trace::{run, run_padded, ExecutionTrace, TraceRow};