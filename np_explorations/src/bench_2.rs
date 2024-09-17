use std::{fs::metadata, os::unix::fs::MetadataExt};

use evm_arithmetization::{
    proof::PublicValues, prover::prove, testing_utils::init_logger, verifier::verify_proof,
    AllRecursiveCircuits, AllStark, StarkConfig,
};
use itertools::Itertools;
use plonky2::{plonk::proof::ProofWithPublicInputs, util::timing::TimingTree};

mod common;
use common::{
    get_generation_inputs_from_json, D, F, KC, PC, STARKY_PROVER_CONFIG, STARKY_VERIFIER_CONFIG,
};
const TMP_PATH: &str = "np_explorations/data/bench_2/tmp.json";

// Configurable parameters
const BENCH_LEVEL_0: bool = false;
// The pow-of-2-trimmed block will be truncated to 1/2^SHRINKING_FACTOR_LOG of
// its original size. For instance, 2 indicates that only 1/4 of the (trimmed)
// block will be used.
const SHRINKING_FACTOR_LOG: usize = 2;

fn main() {
    init_logger();

    let mut block =
        get_generation_inputs_from_json("np_explorations/data/bench_2/block_input.json");
    let mut n_transactions = block.len();

    // Trimming to a power of two to get more accurate measurements
    n_transactions = (n_transactions + 1).next_power_of_two() / 2;
    block.truncate(n_transactions);

    // Shrinking
    n_transactions /= n_transactions / (1 << SHRINKING_FACTOR_LOG);
    block.truncate(n_transactions);

    let all_stark = AllStark::default();
    let fast_starky_config = StarkConfig::standard_fast_config();

    let mut timing_tree = if BENCH_LEVEL_0 {
        ////////////////////////////////////////////////////////////////////////////
        // First measurement: no recursion (7 starky proofs + 1 CTL per transaction)
        log::info!("\n\n******** Level 0: No recursion ********");

        log::info!("Starky config:\n{:?}", STARKY_VERIFIER_CONFIG);

        // Measure prover time
        let block_l0 = block.clone();
        let timer = std::time::Instant::now();
        let mut timing_tree = TimingTree::default();
        let proofs = block_l0
            .into_iter()
            .enumerate()
            .map(|(i, generation_inputs)| {
                log::info!(" * Transaction {i}");
                prove::<F, KC, D>(
                    &all_stark,
                    &STARKY_VERIFIER_CONFIG,
                    generation_inputs,
                    &mut timing_tree,
                    None,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let total_prover_time_l0 = timer.elapsed();

        // Measure verification time
        let proofs_l0 = proofs.clone();

        let timer = std::time::Instant::now();
        for proof in proofs_l0.into_iter() {
            verify_proof(&all_stark, proof, &fast_starky_config).unwrap();
        }
        let total_verifier_time_l0 = timer.elapsed();

        // Compute size
        let mut total_size = 0;

        for proof in proofs.iter() {
            serde_json::to_writer(std::fs::File::create(&TMP_PATH).unwrap(), &proof).unwrap();
            let file_metadata = metadata(&TMP_PATH).unwrap();
            total_size += file_metadata.size();
        }

        log_space_and_time(
            n_transactions,
            total_size as f32,
            total_prover_time_l0,
            total_verifier_time_l0,
        );

        timing_tree
    } else {
        TimingTree::default()
    };

    ////////////////////////////////////////////////////////////////////////////
    // Second measurement: Produce one plonky2 proof per transaction
    log::info!("\n\n******** Level 1: One level of recursion ********");

    log::info!("Starky config:\n{:?}", STARKY_PROVER_CONFIG);

    let prover_state = AllRecursiveCircuits::<F, PC, D>::new(
        &all_stark,
        // TODO what is this? It is related to the starky machines and they say it should "be large
        // enough for your application"
        &[16..25, 9..20, 12..25, 14..25, 9..20, 12..20, 17..30],
        &STARKY_PROVER_CONFIG,
    );

    // Measure prover time
    let block_l1 = block;

    let timer = std::time::Instant::now();

    let root_proofs = block_l1
        .into_iter()
        .map(|generation_inputs| {
            prover_state
                .prove_root(
                    &all_stark,
                    &STARKY_PROVER_CONFIG,
                    generation_inputs,
                    &mut timing_tree,
                    None,
                )
                .unwrap()
        })
        .collect::<Vec<_>>();

    let total_prover_time_l1 = timer.elapsed();

    let proofs_l1 = root_proofs.clone();

    let timer = std::time::Instant::now();
    for proof in proofs_l1.into_iter() {
        prover_state.verify_root(proof.0).unwrap();
    }
    let total_verifier_time_l1 = timer.elapsed();

    let mut total_size = 0;

    for proof in root_proofs.iter() {
        serde_json::to_writer(std::fs::File::create(&TMP_PATH).unwrap(), &proof.0).unwrap();
        let file_metadata = metadata(&TMP_PATH).unwrap();
        total_size += file_metadata.size();
    }

    log_space_and_time(
        n_transactions,
        total_size as f32,
        total_prover_time_l1,
        total_verifier_time_l1,
    );

    ////////////////////////////////////////////////////////////////////////////
    // Third measurement: Aggregate plonky2 proofs in pairs
    log::info!("\n\n******** Level 2: Two levels of recursion ********");

    log::info!("Starky config:\n{:?}", STARKY_PROVER_CONFIG);

    let timer = std::time::Instant::now();

    let aggregated_proofs = aggregate_proofs(&prover_state, root_proofs);

    let total_prover_time_l2 = timer.elapsed() + total_prover_time_l1;

    let timer = std::time::Instant::now();
    for proof in aggregated_proofs.iter() {
        prover_state.verify_aggregation(&proof.0).unwrap();
    }
    let total_verifier_time_l2 = timer.elapsed();

    let mut total_size = 0;

    for proof in aggregated_proofs.iter() {
        serde_json::to_writer(std::fs::File::create(&TMP_PATH).unwrap(), &proof.0).unwrap();
        let file_metadata = metadata(&TMP_PATH).unwrap();
        total_size += file_metadata.size();
    }

    log_space_and_time(
        n_transactions,
        total_size as f32,
        total_prover_time_l2,
        total_verifier_time_l2,
    );

    ////////////////////////////////////////////////////////////////////////////
    // Fourth measurement: Aggregate plonky2 proofs into one
    log::info!("\n\n******** Full aggregation ********");

    let timer = std::time::Instant::now();
    let mut aggregated_proofs = aggregated_proofs;
    while aggregated_proofs.len() > 1 {
        aggregated_proofs = aggregate_proofs(&prover_state, aggregated_proofs);
    }
    let total_prover_time_aggregation = timer.elapsed() + total_prover_time_l2;

    let (last_level_proof, pv) = aggregated_proofs.into_iter().next().unwrap();

    let timer = std::time::Instant::now();
    prover_state.verify_aggregation(&last_level_proof).unwrap();
    let total_verifier_time_aggregation = timer.elapsed();

    serde_json::to_writer(std::fs::File::create(&TMP_PATH).unwrap(), &last_level_proof).unwrap();
    let file_metadata = metadata(&TMP_PATH).unwrap();
    total_size = file_metadata.size();

    log_space_and_time(
        n_transactions,
        total_size as f32,
        total_prover_time_aggregation,
        total_verifier_time_aggregation,
    );

    ////////////////////////////////////////////////////////////////////////////
    // Fifth measurement: Recurse last proof into a block proof
    log::info!("\n\n******** Full recursion ********");

    let timer = std::time::Instant::now();
    let (block_proof, _) = prover_state
        .prove_block(None, &last_level_proof, pv)
        .unwrap();
    let total_prover_time_recursion = timer.elapsed() + total_prover_time_aggregation;

    let timer = std::time::Instant::now();
    prover_state.verify_block(&block_proof).unwrap();
    let total_verifier_time_recursion = timer.elapsed();

    serde_json::to_writer(std::fs::File::create(&TMP_PATH).unwrap(), &block_proof).unwrap();
    let file_metadata = metadata(&TMP_PATH).unwrap();
    total_size = file_metadata.size();

    log_space_and_time(
        n_transactions,
        total_size as f32,
        total_prover_time_recursion,
        total_verifier_time_recursion,
    );
}

pub fn aggregate_proofs(
    prover_state: &AllRecursiveCircuits<F, PC, D>,
    proofs: Vec<(ProofWithPublicInputs<F, PC, D>, PublicValues)>,
) -> Vec<(ProofWithPublicInputs<F, PC, D>, PublicValues)> {
    proofs
        .into_iter()
        .chunks(2)
        .into_iter()
        .map(|c| {
            let ((proof_0, pv_0), (proof_1, pv_1)) = c.into_iter().collect_tuple().unwrap();
            prover_state
                .prove_aggregation(false, &proof_0, pv_0, false, &proof_1, pv_1)
                .unwrap()
        })
        .collect::<Vec<_>>()
}

fn log_space_and_time(
    n_transactions: usize,
    total_size: f32,
    total_prover_time_l2: std::time::Duration,
    total_verifier_time_l2: std::time::Duration,
) {
    log::info!(" - Number of transactions: {:?}", n_transactions);
    log::info!(" - Total proof size: {:?} MB", total_size / 1_000_000_f32);
    log::info!(
        "   Average proof size: {:?} MB",
        total_size / ((n_transactions * 1_000_000) as f32)
    );
    log::info!(" - Total prover time: {:?}", total_prover_time_l2);
    log::info!(
        "   Average prover time: {:?} s.",
        total_prover_time_l2.as_secs_f32() / n_transactions as f32
    );
    log::info!(" - Total verifier time: {:?}", total_verifier_time_l2);
    log::info!(
        "   Average verifier time: {:?} s.",
        total_verifier_time_l2.as_secs_f32() / n_transactions as f32
    );
}
