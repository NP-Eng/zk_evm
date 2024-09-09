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
        bench_arithmetic_operation(c, op, op_str, config.clone(), &config_str);
    }
}

fn bench_arithmetic_operation(
    c: &mut Criterion,
    op: Operation,
    op_str: &str,
    config: StarkConfig,
    config_str: &str,
) {
    let stark = ArithmeticStark::<F, 2>::default();
    let tag = format!("ArithmeticStark ({:?}) with {}", op_str, config_str);
    let trace = stark.generate_trace(vec![op]);
    bench_stark::<F, C, S, D>(c, stark, trace, config, &tag);
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_arithmetic_stark::<D, F>
}
criterion_main!(benches);
