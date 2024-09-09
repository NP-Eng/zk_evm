use criterion::Criterion;
use ethereum_types::U256;
use plonky2::{
    field::{extension::Extendable, polynomial::PolynomialValues},
    fri::oracle::PolynomialBatch,
    hash::hash_types::RichField,
    iop::challenger::Challenger,
    plonk::config::{AlgebraicHasher, GenericConfig},
    util::timing::TimingTree,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use starky::{
    cross_table_lookup::{CtlData, CtlZData},
    lookup::{Column, Filter, GrandProductChallenge, GrandProductChallengeSet},
    prover::prove,
    stark::Stark,
};

use crate::prover::prove_single_table;
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

pub fn bench_ctl_stark<F, C, S, const D: usize>(
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
    let trace_commitments = PolynomialBatch::<F, C, D>::from_values(
        trace.clone(),
        config.fri_config.rate_bits,
        false,
        config.fri_config.cap_height,
        &mut TimingTree::default(),
        None,
    );

    let degree = 1 << trace_commitments.degree_log;

    // Fake CTL data.
    let ctl_z_data = CtlZData::new(
        vec![PolynomialValues::zero(degree)],
        PolynomialValues::zero(degree),
        GrandProductChallenge {
            beta: F::ZERO,
            gamma: F::ZERO,
        },
        vec![],
        vec![Filter::new_simple(Column::constant(F::ZERO))],
    );

    let ctl_data = CtlData {
        zs_columns: vec![ctl_z_data.clone(); config.num_challenges],
    };

    c.bench_function(tag, |b| {
        b.iter(|| {
            prove_single_table(
                &stark,
                &config,
                &trace,
                &trace_commitments,
                &ctl_data,
                &GrandProductChallengeSet {
                    challenges: vec![ctl_z_data.challenge; config.num_challenges],
                },
                &mut Challenger::new(),
                &mut TimingTree::default(),
                None,
            )
            .unwrap();
        })
    });
}

pub fn rand_u256() -> U256 {
    U256::from(ChaCha20Rng::from_entropy().gen::<[u8; 32]>())
}
