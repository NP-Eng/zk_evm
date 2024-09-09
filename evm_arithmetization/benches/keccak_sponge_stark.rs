use criterion::{criterion_group, criterion_main, Criterion};
use evm_arithmetization::{
    bench_utils::bench_stark,
    keccak_sponge::keccak_sponge_stark::{KeccakSpongeOp, KeccakSpongeStark},
    memory::segments::Segment,
    stark_config,
    witness::memory::MemoryAddress,
};
use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField},
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    hash::hash_types::RichField,
    plonk::config::PoseidonGoldilocksConfig,
    util::timing::TimingTree,
};
use rand::Rng;
use starky::config::StarkConfig;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
type S = KeccakSpongeStark<F, D>;

fn bench_keccak_sponge_stark<const D: usize, F>(c: &mut Criterion)
where
    F: RichField + Extendable<D>,
{
    let (config_str, config) = stark_config!(4, 5);

    let mut op = KeccakSpongeOp {
        base_address: MemoryAddress::new(0, Segment::Code, 0),
        timestamp: 0,
        input: vec![],
    };

    let rng = &mut rand::thread_rng();

    for input_length_bits in 0..=16 {
        op.input = (0..(1 << input_length_bits))
            .map(|_| rng.gen::<u8>())
            .collect();
        bench_keccak_operation(c, op.clone(), config.clone(), &config_str);
    }
}

fn bench_keccak_operation(
    c: &mut Criterion,
    op: KeccakSpongeOp,
    config: StarkConfig,
    config_str: &str,
) {
    let stark = S::default();
    let tag = format!(
        "KeccakSpongeStark (input length: {}) with {}",
        op.input.len(),
        config_str
    );
    let trace = stark.generate_trace(vec![op], 1024, &mut TimingTree::default());
    bench_stark::<F, C, S, D>(c, stark, trace, config, &tag);
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_keccak_sponge_stark::<D, F>
}
criterion_main!(benches);
