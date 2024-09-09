use criterion::Criterion;
use ethereum_types::U256;
use plonky2::{
    field::{extension::Extendable, polynomial::PolynomialValues},
    hash::hash_types::RichField,
    plonk::config::{AlgebraicHasher, GenericConfig},
    util::timing::TimingTree,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use starky::{prover::prove, stark::Stark};

use crate::StarkConfig;

#[macro_export]
macro_rules! stark_config {
    ($arity:expr, $fri_reduction_arity:expr) => {
        (
            format!(
                "ConstantArityBits({}, {}){}",
                $arity,
                $fri_reduction_arity,
                if $arity == 4 && $fri_reduction_arity == 5 {
                    " aka Standard"
                } else {
                    ""
                }
            ),
            StarkConfig::new(
                100,
                2,
                FriConfig {
                    rate_bits: 1,
                    cap_height: 4,
                    proof_of_work_bits: 16,
                    reduction_strategy: FriReductionStrategy::ConstantArityBits(
                        $arity,
                        $fri_reduction_arity,
                    ),
                    num_query_rounds: 84,
                },
            ),
        )
    };
}

pub fn bench_stark<F, C, S, const D: usize>(
    c: &mut Criterion,
    stark: S,
    trace: Vec<PolynomialValues<F>>,
    config: StarkConfig,
    tag: &str,
) where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
    S: Stark<F, D> + Clone,
{
    c.bench_function(tag, |b| {
        b.iter(|| {
            prove::<F, C, S, D>(
                stark.clone(),
                &config,
                trace.clone(),
                &[],
                &mut TimingTree::default(),
            )
            .unwrap()
        })
    });
}

pub fn rand_u256() -> U256 {
    U256::from(ChaCha20Rng::from_entropy().gen::<[u8; 32]>())
}
