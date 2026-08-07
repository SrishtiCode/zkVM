// `cargo run -p isa --example power`
use field::{Field, ToyField};
use isa::{run, Instruction};

/// Registers: r0 = result (accumulator), r1 = base, r2 = exponent
/// (counter), r3 = the constant -1.
fn power_program(base: u64, exponent: u64) -> Vec<Instruction<ToyField>> {
    use Instruction::*;
    vec![
        /* 0 */ LoadImm { reg: 0, imm: ToyField::from_u64(1) },        // result = 1
        /* 1 */ LoadImm { reg: 1, imm: ToyField::from_u64(base) },     // base
        /* 2 */ LoadImm { reg: 2, imm: ToyField::from_u64(exponent) }, // counter
        /* 3 */ LoadImm { reg: 3, imm: -ToyField::one() },             // -1 constant
        // loop:
        /* 4 */ Jnz { reg: 2, target: 6 },
        /* 5 */ Jmp { target: 9 },
        /* 6 */ Mul { dst: 0, a: 0, b: 1 }, // result *= base
        /* 7 */ Add { dst: 2, a: 2, b: 3 }, // counter += (-1)
        /* 8 */ Jmp { target: 4 },
        /* 9 */ Halt,
    ]
}

fn expected_power(base: u64, exponent: u64) -> u64 {
    let modulus = ToyField::modulus();
    let mut result = 1u64 % modulus;
    for _ in 0..exponent {
        result = (result * (base % modulus)) % modulus;
    }
    result
}

fn main() {
    let (base, exponent) = (3u64, 6u64);
    let program = power_program(base, exponent);
    let trace = run(&program, vec![], 200);

    println!("{}", trace.pretty_print());
    println!("{base}^{exponent} mod 97 = {}", trace.final_registers[0].to_canonical_u64());

    assert_eq!(trace.final_registers[0], ToyField::from_u64(expected_power(base, exponent)));
    println!("matches the expected value.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_matches_reference_for_several_inputs() {
        for &(base, exp) in &[(2u64, 0u64), (2, 1), (3, 6), (5, 10), (7, 4)] {
            let trace = run(&power_program(base, exp), vec![], 500);
            assert_eq!(
                trace.final_registers[0],
                ToyField::from_u64(expected_power(base, exp)),
                "mismatch for {base}^{exp}"
            );
        }
    }
}

/*
└─$ cargo run -p isa --example power
   Compiling isa v0.1.0 (/home/srishti/Work/zkVM/zkVM/crates/isa)
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.20s
     Running `target/debug/examples/power`
cycle   pc  instruction                   registers
    0    0  loadimm r0, 1                 [0, 0, 0, 0, 0, 0]
    1    1  loadimm r1, 3                 [1, 0, 0, 0, 0, 0]
    2    2  loadimm r2, 6                 [1, 3, 0, 0, 0, 0]
    3    3  loadimm r3, 96                [1, 3, 6, 0, 0, 0]
    4    4  jnz     r2, 6                 [1, 3, 6, 96, 0, 0]
    5    6  mul     r0, r0, r1            [1, 3, 6, 96, 0, 0]
    6    7  add     r2, r2, r3            [3, 3, 6, 96, 0, 0]
    7    8  jmp     4                     [3, 3, 5, 96, 0, 0]
    8    4  jnz     r2, 6                 [3, 3, 5, 96, 0, 0]
    9    6  mul     r0, r0, r1            [3, 3, 5, 96, 0, 0]
   10    7  add     r2, r2, r3            [9, 3, 5, 96, 0, 0]
   11    8  jmp     4                     [9, 3, 4, 96, 0, 0]
   12    4  jnz     r2, 6                 [9, 3, 4, 96, 0, 0]
   13    6  mul     r0, r0, r1            [9, 3, 4, 96, 0, 0]
   14    7  add     r2, r2, r3            [27, 3, 4, 96, 0, 0]
   15    8  jmp     4                     [27, 3, 3, 96, 0, 0]
   16    4  jnz     r2, 6                 [27, 3, 3, 96, 0, 0]
   17    6  mul     r0, r0, r1            [27, 3, 3, 96, 0, 0]
   18    7  add     r2, r2, r3            [81, 3, 3, 96, 0, 0]
   19    8  jmp     4                     [81, 3, 2, 96, 0, 0]
   20    4  jnz     r2, 6                 [81, 3, 2, 96, 0, 0]
   21    6  mul     r0, r0, r1            [81, 3, 2, 96, 0, 0]
   22    7  add     r2, r2, r3            [49, 3, 2, 96, 0, 0]
   23    8  jmp     4                     [49, 3, 1, 96, 0, 0]
   24    4  jnz     r2, 6                 [49, 3, 1, 96, 0, 0]
   25    6  mul     r0, r0, r1            [49, 3, 1, 96, 0, 0]
   26    7  add     r2, r2, r3            [50, 3, 1, 96, 0, 0]
   27    8  jmp     4                     [50, 3, 0, 96, 0, 0]
   28    4  jnz     r2, 6                 [50, 3, 0, 96, 0, 0]
   29    5  jmp     9                     [50, 3, 0, 96, 0, 0]
   30    9  halt                          [50, 3, 0, 96, 0, 0]

3^6 mod 97 = 50
matches the expected value.
                              
*/