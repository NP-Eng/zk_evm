use criterion::{criterion_group, criterion_main, Criterion};
use evm_arithmetization::{
    bench_utils::bench_ctl_stark, keccak::keccak_stark::KeccakStark, stark_config,
};
use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField},
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    hash::hash_types::RichField,
    plonk::config::PoseidonGoldilocksConfig,
    util::timing::TimingTree,
};
use starky::config::StarkConfig;

const BENCH_MIN_ROWS: usize = 8;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
type S = KeccakStark<F, D>;

fn bench_keccak_stark<const D: usize, F>(c: &mut Criterion)
where
    F: RichField + Extendable<D>,
{
    let (config_str, config) = stark_config!(4, 5);

    for input_length_bits in 0..=16 {
        bench_keccak_stark_with_input_length(c, input_length_bits, config.clone(), &config_str);
    }
}

fn bench_keccak_stark_with_input_length(
    c: &mut Criterion,
    input_length_bits: usize,
    config: StarkConfig,
    config_str: &str,
) {
    let stark = S::default();

    // Keccak takes 25 64-bit words as input
    let input: Vec<([u64; 25], usize)> = (0..(1 << input_length_bits))
        .map(|_| (rand::random(), 0))
        .collect();

    let tag = format!(
        "KeccakStark (input length: {}) with {}",
        input_length_bits, config_str
    );
    let trace = stark.generate_trace(input, BENCH_MIN_ROWS, &mut TimingTree::default());
    bench_ctl_stark::<F, C, S, D>(c, stark, trace, config, &tag);
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_keccak_stark::<D, F>
}
criterion_main!(benches);
