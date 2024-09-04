use std::marker::PhantomData;

use plonky2::{
    field::extension::Extendable,
    hash::hash_types::RichField,
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::CircuitConfig,
        config::{AlgebraicHasher, GenericConfig},
    },
    util::serialization::{DefaultGateSerializer, DefaultGeneratorSerializer},
};
use starky::{
    constraint_consumer::RecursiveConstraintConsumer, evaluation_frame::StarkEvaluationFrame,
    stark::Stark,
};
#[cfg(test)]
use {
    crate::{
        arithmetic::arithmetic_stark::ArithmeticStark,
        byte_packing::byte_packing_stark::BytePackingStark, cpu::cpu_stark::CpuStark,
        keccak::keccak_stark::KeccakStark, keccak_sponge::keccak_sponge_stark::KeccakSpongeStark,
        logic::LogicStark, memory::memory_stark::MemoryStark,
    },
    plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig},
};

fn get_stark_size<F, C, const D: usize, S>(stark: S) -> usize
where
    S: Stark<F, D>,
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
{
    let mut builder = CircuitBuilder::new(CircuitConfig::standard_recursion_config());

    let locals_t = builder.add_virtual_extension_targets(S::COLUMNS);
    let nexts_t = builder.add_virtual_extension_targets(S::COLUMNS);
    let alphas_t = builder.add_virtual_targets(1);
    let z_last_t = builder.add_virtual_extension_target();
    let lagrange_first_t = builder.add_virtual_extension_target();
    let lagrange_last_t = builder.add_virtual_extension_target();

    let mut consumer = RecursiveConstraintConsumer::<F, D>::new(
        builder.zero_extension(),
        alphas_t,
        z_last_t,
        lagrange_first_t,
        lagrange_last_t,
    );

    let ef = S::EvaluationFrameTarget::from_values(&locals_t, &nexts_t, &[]);

    // Evaluate constraints.
    stark.eval_ext_circuit(&mut builder, &ef, &mut consumer);

    let generator_serializer = DefaultGeneratorSerializer::<C, D> {
        _phantom: PhantomData::<C>,
    };

    builder
        .build::<C>()
        .to_bytes(&DefaultGateSerializer {}, &generator_serializer)
        .unwrap()
        .len()
}

#[test]
fn test_circuit_sizes() {
    const D: usize = 2;
    type F = GoldilocksField;
    type C = PoseidonGoldilocksConfig;

    let size = get_stark_size::<F, C, D, _>(ArithmeticStark::<F, D>::default());
    println!("ArithmeticStark size: {}", size);

    let size = get_stark_size::<F, C, D, _>(BytePackingStark::<F, D>::default());
    println!("BytePackingStark size: {}", size);

    let size = get_stark_size::<F, C, D, _>(CpuStark::<F, D>::default());
    println!("CpuStark size: {}", size);

    let size = get_stark_size::<F, C, D, _>(KeccakStark::<F, D>::default());
    println!("KeccakStark size: {}", size);

    let size = get_stark_size::<F, C, D, _>(KeccakSpongeStark::<F, D>::default());
    println!("KeccakSpongeStark size: {}", size);

    let size = get_stark_size::<F, C, D, _>(LogicStark::<F, D>::default());
    println!("LogicStark size: {}", size);

    let size = get_stark_size::<F, C, D, _>(MemoryStark::<F, D>::default());
    println!("MemoryStark size: {}", size);
}
