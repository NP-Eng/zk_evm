use criterion::{criterion_group, criterion_main, Criterion};
use evm_arithmetization::{
    bench_utils::{bench_ctl_stark, rand_u256},
    logic::{LogicStark, Op, Operation},
    stark_config,
};
use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField},
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    hash::hash_types::RichField,
    plonk::config::PoseidonGoldilocksConfig,
    util::timing::TimingTree,
};
use starky::config::StarkConfig;

const BENCH_MIN_ROWS: usize = 1 << 10;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
type S = LogicStark<F, D>;

macro_rules! logic_operations {
    ($($op:ident),*) => {
        vec![
            $(Operation::new(Op::$op, rand_u256(), rand_u256())),*
        ]
    };
}

fn bench_logic_stark<const D: usize, F>(c: &mut Criterion)
where
    F: RichField + Extendable<D>,
{
    let (config_str, config) = stark_config!(4, 5);

    for (op, op_str) in logic_operations!(Or, And, Xor)
        .into_iter()
        .zip(["Or", "And", "Xor"])
    {
        bench_logic_operation(c, op, op_str, config.clone(), &config_str);
    }
}

fn bench_logic_operation(
    c: &mut Criterion,
    op: Operation,
    op_str: &str,
    config: StarkConfig,
    config_str: &str,
) {
    let stark = LogicStark::<F, 2>::default();
    let tag = format!("LogicStark ({:?}) with {}", op_str, config_str);
    let trace = stark.generate_trace(vec![op], BENCH_MIN_ROWS, &mut TimingTree::default());
    bench_ctl_stark::<F, C, S, D>(c, stark, trace, config, &tag);
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_logic_stark::<D, F>
}
criterion_main!(benches);
