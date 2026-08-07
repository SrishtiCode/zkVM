// `cargo run -p isa --example fibonacci`
use field::{Field, ToyField};
use isa::{run, Instruction};

/// Registers:
///   r0 = a (F(i))         r1 = b (F(i+1))       r2 = loop counter
///   r3 = temp (new b)     r4 = the constant -1  r5 = the constant 0
fn fibonacci_program(n: u64) -> Vec<Instruction<ToyField>> {
    use Instruction::*;
    vec![
        /*  0 */ LoadImm { reg: 0, imm: ToyField::from_u64(0) }, // a = F(0)
        /*  1 */ LoadImm { reg: 1, imm: ToyField::from_u64(1) }, // b = F(1)
        /*  2 */ LoadImm { reg: 2, imm: ToyField::from_u64(n) }, // counter = n
        /*  3 */ LoadImm { reg: 4, imm: -ToyField::one() },      // -1 constant
        /*  4 */ LoadImm { reg: 5, imm: ToyField::from_u64(0) }, // 0 constant
        // loop:
        /*  5 */ Jnz { reg: 2, target: 7 }, // counter != 0 -> keep looping
        /*  6 */ Jmp { target: 12 },        // counter == 0 -> exit
        /*  7 */ Add { dst: 3, a: 0, b: 1 }, // temp = a + b
        /*  8 */ Add { dst: 0, a: 1, b: 5 }, // a = b  (copy, via + 0)
        /*  9 */ Add { dst: 1, a: 3, b: 5 }, // b = temp (copy, via + 0)
        /* 10 */ Add { dst: 2, a: 2, b: 4 }, // counter += (-1)
        /* 11 */ Jmp { target: 5 },
        /* 12 */ Halt,
    ]
}

fn expected_fib(n: u64) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 0..n {
        let next = a + b;
        a = b;
        b = next;
    }
    a
}

fn main() {
    let n = 10u64;
    let program = fibonacci_program(n);
    let trace = run(&program, vec![], 200);

    println!("{}", trace.pretty_print());
    println!("fib({n}) = {}", trace.final_registers[0].to_canonical_u64());

    assert_eq!(trace.final_registers[0], ToyField::from_u64(expected_fib(n)));
    println!("matches the expected value ({}).", expected_fib(n));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fibonacci_matches_reference_for_several_n() {
        for &n in &[0u64, 1, 2, 5, 10, 11] {
            let trace = run(&fibonacci_program(n), vec![], 500);
            assert_eq!(
                trace.final_registers[0],
                ToyField::from_u64(expected_fib(n)),
                "mismatch for n={n}"
            );
        }
    }
}

/*
 cargo run -p isa --example fibonacci
   Compiling isa v0.1.0 (/home/srishti/Work/zkVM/zkVM/crates/isa)
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.21s
     Running `target/debug/examples/fibonacci`
cycle   pc  instruction                   registers
    0    0  loadimm r0, 0                 [0, 0, 0, 0, 0, 0]
    1    1  loadimm r1, 1                 [0, 0, 0, 0, 0, 0]
    2    2  loadimm r2, 10                [0, 1, 0, 0, 0, 0]
    3    3  loadimm r4, 96                [0, 1, 10, 0, 0, 0]
    4    4  loadimm r5, 0                 [0, 1, 10, 0, 96, 0]
    5    5  jnz     r2, 7                 [0, 1, 10, 0, 96, 0]
    6    7  add     r3, r0, r1            [0, 1, 10, 0, 96, 0]
    7    8  add     r0, r1, r5            [0, 1, 10, 1, 96, 0]
    8    9  add     r1, r3, r5            [1, 1, 10, 1, 96, 0]
    9   10  add     r2, r2, r4            [1, 1, 10, 1, 96, 0]
   10   11  jmp     5                     [1, 1, 9, 1, 96, 0]
   11    5  jnz     r2, 7                 [1, 1, 9, 1, 96, 0]
   12    7  add     r3, r0, r1            [1, 1, 9, 1, 96, 0]
   13    8  add     r0, r1, r5            [1, 1, 9, 2, 96, 0]
   14    9  add     r1, r3, r5            [1, 1, 9, 2, 96, 0]
   15   10  add     r2, r2, r4            [1, 2, 9, 2, 96, 0]
   16   11  jmp     5                     [1, 2, 8, 2, 96, 0]
   17    5  jnz     r2, 7                 [1, 2, 8, 2, 96, 0]
   18    7  add     r3, r0, r1            [1, 2, 8, 2, 96, 0]
   19    8  add     r0, r1, r5            [1, 2, 8, 3, 96, 0]
   20    9  add     r1, r3, r5            [2, 2, 8, 3, 96, 0]
   21   10  add     r2, r2, r4            [2, 3, 8, 3, 96, 0]
   22   11  jmp     5                     [2, 3, 7, 3, 96, 0]
   23    5  jnz     r2, 7                 [2, 3, 7, 3, 96, 0]
   24    7  add     r3, r0, r1            [2, 3, 7, 3, 96, 0]
   25    8  add     r0, r1, r5            [2, 3, 7, 5, 96, 0]
   26    9  add     r1, r3, r5            [3, 3, 7, 5, 96, 0]
   27   10  add     r2, r2, r4            [3, 5, 7, 5, 96, 0]
   28   11  jmp     5                     [3, 5, 6, 5, 96, 0]
   29    5  jnz     r2, 7                 [3, 5, 6, 5, 96, 0]
   30    7  add     r3, r0, r1            [3, 5, 6, 5, 96, 0]
   31    8  add     r0, r1, r5            [3, 5, 6, 8, 96, 0]
   32    9  add     r1, r3, r5            [5, 5, 6, 8, 96, 0]
   33   10  add     r2, r2, r4            [5, 8, 6, 8, 96, 0]
   34   11  jmp     5                     [5, 8, 5, 8, 96, 0]
   35    5  jnz     r2, 7                 [5, 8, 5, 8, 96, 0]
   36    7  add     r3, r0, r1            [5, 8, 5, 8, 96, 0]
   37    8  add     r0, r1, r5            [5, 8, 5, 13, 96, 0]
   38    9  add     r1, r3, r5            [8, 8, 5, 13, 96, 0]
   39   10  add     r2, r2, r4            [8, 13, 5, 13, 96, 0]
   40   11  jmp     5                     [8, 13, 4, 13, 96, 0]
   41    5  jnz     r2, 7                 [8, 13, 4, 13, 96, 0]
   42    7  add     r3, r0, r1            [8, 13, 4, 13, 96, 0]
   43    8  add     r0, r1, r5            [8, 13, 4, 21, 96, 0]
   44    9  add     r1, r3, r5            [13, 13, 4, 21, 96, 0]
   45   10  add     r2, r2, r4            [13, 21, 4, 21, 96, 0]
   46   11  jmp     5                     [13, 21, 3, 21, 96, 0]
   47    5  jnz     r2, 7                 [13, 21, 3, 21, 96, 0]
   48    7  add     r3, r0, r1            [13, 21, 3, 21, 96, 0]
   49    8  add     r0, r1, r5            [13, 21, 3, 34, 96, 0]
   50    9  add     r1, r3, r5            [21, 21, 3, 34, 96, 0]
   51   10  add     r2, r2, r4            [21, 34, 3, 34, 96, 0]
   52   11  jmp     5                     [21, 34, 2, 34, 96, 0]
   53    5  jnz     r2, 7                 [21, 34, 2, 34, 96, 0]
   54    7  add     r3, r0, r1            [21, 34, 2, 34, 96, 0]
   55    8  add     r0, r1, r5            [21, 34, 2, 55, 96, 0]
   56    9  add     r1, r3, r5            [34, 34, 2, 55, 96, 0]
   57   10  add     r2, r2, r4            [34, 55, 2, 55, 96, 0]
   58   11  jmp     5                     [34, 55, 1, 55, 96, 0]
   59    5  jnz     r2, 7                 [34, 55, 1, 55, 96, 0]
   60    7  add     r3, r0, r1            [34, 55, 1, 55, 96, 0]
   61    8  add     r0, r1, r5            [34, 55, 1, 89, 96, 0]
   62    9  add     r1, r3, r5            [55, 55, 1, 89, 96, 0]
   63   10  add     r2, r2, r4            [55, 89, 1, 89, 96, 0]
   64   11  jmp     5                     [55, 89, 0, 89, 96, 0]
   65    5  jnz     r2, 7                 [55, 89, 0, 89, 96, 0]
   66    6  jmp     12                    [55, 89, 0, 89, 96, 0]
   67   12  halt                          [55, 89, 0, 89, 96, 0]

fib(10) = 55
matches the expected value (55).
*/