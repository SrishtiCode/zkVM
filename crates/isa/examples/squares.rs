// `cargo run -p isa --example squares`

use field::{Field, ToyField};
use isa::{run, Instruction};

const COUNT: u64 = 8;

/// Registers: r0 = i (1..=COUNT), r1 = i*i (scratch).
fn squares_program() -> Vec<Instruction<ToyField>> {
    use Instruction::*;
    let mut program = Vec::new();
    for i in 1..=COUNT {
        program.push(LoadImm { reg: 0, imm: ToyField::from_u64(i) });
        program.push(Mul { dst: 1, a: 0, b: 0 });
        program.push(Store { addr: (i - 1) as usize, reg: 1 });
    }
    program.push(Instruction::Halt);
    program
}

fn main() {
    let program = squares_program();
    let trace = run(&program, vec![], 200);

    println!("{}", trace.pretty_print());
    let squares: Vec<u64> =
        trace.final_memory[..COUNT as usize].iter().map(|v| v.to_canonical_u64()).collect();
    println!("squares: {squares:?}");

    for i in 1..=COUNT {
        let expected = (i * i) % ToyField::modulus();
        assert_eq!(trace.final_memory[(i - 1) as usize], ToyField::from_u64(expected));
    }
    println!("all {COUNT} squares match the expected values.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squares_are_stored_at_the_expected_addresses() {
        let trace = run(&squares_program(), vec![], 200);
        for i in 1..=COUNT {
            let expected = (i * i) % ToyField::modulus();
            assert_eq!(trace.final_memory[(i - 1) as usize], ToyField::from_u64(expected));
        }
    }
}

/*
└─$ cargo run -p isa --example squares
   Compiling isa v0.1.0 (/home/srishti/Work/zkVM/zkVM/crates/isa)
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.44s
     Running `target/debug/examples/squares`
cycle   pc  instruction                   registers
    0    0  loadimm r0, 1                 [0, 0, 0, 0, 0, 0]
    1    1  mul     r1, r0, r0            [1, 0, 0, 0, 0, 0]
    2    2  store   [0], r1               [1, 1, 0, 0, 0, 0]
    3    3  loadimm r0, 2                 [1, 1, 0, 0, 0, 0]
    4    4  mul     r1, r0, r0            [2, 1, 0, 0, 0, 0]
    5    5  store   [1], r1               [2, 4, 0, 0, 0, 0]
    6    6  loadimm r0, 3                 [2, 4, 0, 0, 0, 0]
    7    7  mul     r1, r0, r0            [3, 4, 0, 0, 0, 0]
    8    8  store   [2], r1               [3, 9, 0, 0, 0, 0]
    9    9  loadimm r0, 4                 [3, 9, 0, 0, 0, 0]
   10   10  mul     r1, r0, r0            [4, 9, 0, 0, 0, 0]
   11   11  store   [3], r1               [4, 16, 0, 0, 0, 0]
   12   12  loadimm r0, 5                 [4, 16, 0, 0, 0, 0]
   13   13  mul     r1, r0, r0            [5, 16, 0, 0, 0, 0]
   14   14  store   [4], r1               [5, 25, 0, 0, 0, 0]
   15   15  loadimm r0, 6                 [5, 25, 0, 0, 0, 0]
   16   16  mul     r1, r0, r0            [6, 25, 0, 0, 0, 0]
   17   17  store   [5], r1               [6, 36, 0, 0, 0, 0]
   18   18  loadimm r0, 7                 [6, 36, 0, 0, 0, 0]
   19   19  mul     r1, r0, r0            [7, 36, 0, 0, 0, 0]
   20   20  store   [6], r1               [7, 49, 0, 0, 0, 0]
   21   21  loadimm r0, 8                 [7, 49, 0, 0, 0, 0]
   22   22  mul     r1, r0, r0            [8, 49, 0, 0, 0, 0]
   23   23  store   [7], r1               [8, 64, 0, 0, 0, 0]
   24   24  halt                          [8, 64, 0, 0, 0, 0]

squares: [1, 4, 9, 16, 25, 36, 49, 64]
all 8 squares match the expected values.
*/