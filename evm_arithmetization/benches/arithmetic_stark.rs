use criterion::{criterion_group, criterion_main, Criterion};
use evm_arithmetization::{
    arithmetic::{arithmetic_stark::ArithmeticStark, BinaryOperator, Operation, TernaryOperator},
    bench_utils::{bench_stark, rand_u256},
    stark_config,
};
use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField},
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    hash::hash_types::RichField,
    plonk::config::PoseidonGoldilocksConfig,
};
use starky::config::StarkConfig;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
type S = ArithmeticStark<F, D>;

macro_rules! binary_operations {
    ($($op:ident),*) => {
        vec![
            $(Operation::binary(BinaryOperator::$op, rand_u256(), rand_u256())),*
        ]
    };
}

macro_rules! ternary_operations {
    ($($op:ident),*) => {
        vec![
            $(Operation::ternary(TernaryOperator::$op, rand_u256(), rand_u256(), rand_u256())),*
        ]
    };
}

fn bench_arithmetic_stark<const D: usize, F>(c: &mut Criterion)
where
    F: RichField + Extendable<D>,
{
    let (config_str, config) = stark_config!(4, 5);

    let ops = binary_operations!(Add, Mul, Sub, Div, Mod)
        .into_iter()
        .chain(ternary_operations!(AddMod, MulMod, SubMod));

    let op_strs = [
        "Add", "Mul", "Sub", "Div", "Mod", "AddMod", "MulMod", "SubMod",
    ];

    for (op, op_str) in ops.zip(op_strs) {
        bench_arithmetic_operations(c, vec![op], op_str, config.clone(), &config_str);
    }
}

// The lookup which checks for correct base-2^16 representation of operands
// occurs in the trace table, which therefore needs to have full column
// dedicated to the lookup table 0, 1, ..., 2^16 - 1. This explains the minimum
// number of rows (i.e. interpolating-polynomial degree plus 1) in the trace of
// the Arithmetic STARK. We test that performing <= 2^16 binary operations
// (which require one row each) results in a 2^16-row table, and performing
// 2^16 + 1 binary operations already requires a 2^17-row table
fn bench_binary_large<const D: usize, F>(c: &mut Criterion)
where
    F: RichField + Extendable<D>,
{
    let (config_str, config) = stark_config!(4, 5);

    let ops_half_below = (0..(1 << 15))
        .map(|_| binary_operations!(Add))
        .flatten()
        .collect::<Vec<_>>();

    bench_arithmetic_operations(
        c,
        ops_half_below,
        "Add (half below)",
        config.clone(),
        &config_str,
    );

    let ops_exact = (0..(1 << 16))
        .map(|_| binary_operations!(Add))
        .flatten()
        .collect::<Vec<_>>();

    bench_arithmetic_operations(c, ops_exact, "Add (exact)", config.clone(), &config_str);

    let ops_just_above = (0..(1 << 16) + 1)
        .map(|_| binary_operations!(Add))
        .flatten()
        .collect::<Vec<_>>();

    bench_arithmetic_operations(
        c,
        ops_just_above,
        "Add (just above)",
        config.clone(),
        &config_str,
    );

    let ops_twice_above = (0..(1 << 17))
        .map(|_| binary_operations!(Add))
        .flatten()
        .collect::<Vec<_>>();

    bench_arithmetic_operations(c, ops_twice_above, "Add (twice above)", config, &config_str);
}

// (Cf. explanation to `bench_binary_large`). We test that performing <= 2^15
// ternary operations (which require two rows each) results in a 2^16-row table,
// and performing 2^15 + 1 ternary operations already requires a 2^17-row table
fn bench_ternary_large<const D: usize, F>(c: &mut Criterion) {
    let (config_str, config) = stark_config!(4, 5);

    let ops_half_below = (0..(1 << 14))
        .map(|_| ternary_operations!(AddMod))
        .flatten()
        .collect::<Vec<_>>();

    bench_arithmetic_operations(
        c,
        ops_half_below,
        "AddMod (half below)",
        config.clone(),
        &config_str,
    );

    let ops_exact = (0..(1 << 15))
        .map(|_| ternary_operations!(AddMod))
        .flatten()
        .collect::<Vec<_>>();

    bench_arithmetic_operations(c, ops_exact, "AddMod (exact)", config.clone(), &config_str);

    let ops_just_above = (0..(1 << 15) + 1)
        .map(|_| ternary_operations!(AddMod))
        .flatten()
        .collect::<Vec<_>>();

    bench_arithmetic_operations(
        c,
        ops_just_above,
        "AddMod (just above)",
        config.clone(),
        &config_str,
    );

    let ops_twice_above = (0..(1 << 16))
        .map(|_| ternary_operations!(AddMod))
        .flatten()
        .collect::<Vec<_>>();

    bench_arithmetic_operations(
        c,
        ops_twice_above,
        "AddMod (twice above)",
        config,
        &config_str,
    );
}

fn bench_arithmetic_operations(
    c: &mut Criterion,
    ops: Vec<Operation>,
    op_str: &str,
    config: StarkConfig,
    config_str: &str,
) {
    let stark = ArithmeticStark::<F, 2>::default();
    let tag = format!("ArithmeticStark ({:?}) with {}", op_str, config_str);
    let trace = stark.generate_trace(ops);
    println!("Trace length (number of polys): {}", trace.len());
    println!(
        "Trace width (number of values per poly): {}",
        trace[0].len()
    );
    bench_stark::<F, C, S, D>(c, stark, trace, config, &tag);
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_arithmetic_stark::<D, F>, bench_binary_large::<D, F>, bench_ternary_large::<D, F>
}

criterion_main!(benches);
