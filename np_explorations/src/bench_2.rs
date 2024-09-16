use std::{fs::metadata, os::unix::fs::MetadataExt};

use evm_arithmetization::{
    prover::prove, verifier::verify_proof, AllRecursiveCircuits, AllStark, StarkConfig,
};
use itertools::Itertools;
use plonky2::util::timing::TimingTree;

mod common;
use common::{
    get_generation_inputs_from_json, D, F, KC, PC, STARKY_PROVER_CONFIG, STARKY_VERIFIER_CONFIG,
};
const TMP_PATH: &str = "np_explorations/data/bench_2/tmp.json";

fn main() {
    let mut block =
        get_generation_inputs_from_json("np_explorations/data/bench_2/block_input.json");
    let mut n_transactions = block.len();

    // Triming to multiple-of-2 length for simplicity
    block.truncate(n_transactions - (n_transactions % 2));
    n_transactions = block.len();

    let all_stark = AllStark::default();
    let fast_starky_config = StarkConfig::standard_fast_config();

    // First measurement: no recursion (7 starky proofs + 1 CTL per transaction)
    println!("\n\n******** Level 0: No recursion ********");

    println!("Starky config:\n{:?}", STARKY_VERIFIER_CONFIG);

    // Measure prover time
    let block_l0 = block.clone();
    let timer = std::time::Instant::now();
    let mut timing_tree = TimingTree::default();
    let proofs = block_l0
        .into_iter()
        .enumerate()
        .map(|(i, generation_inputs)| {
            println!(" * Transaction {i}");
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
        let metadata = metadata(&TMP_PATH).unwrap();
        total_size += metadata.size();
    }

    let total_size = total_size as f32;

    log::info!(" - Number of transactions: {:?}", n_transactions);
    log::info!(" - Total proof size: {:?} MB", total_size / 1_000_000_f32);
    log::info!(
        "   Average proof size: {:?} MB",
        total_size / ((n_transactions * 1_000_000) as f32)
    );
    log::info!(" - Total prover time: {:?}", total_prover_time_l0);
    log::info!(
        "   Average prover time: {:?} s.",
        total_prover_time_l0.as_secs_f32() / n_transactions as f32
    );
    log::info!(" - Total verifier time: {:?}", total_verifier_time_l0);
    log::info!(
        "   Average verifier time: {:?} s.",
        total_verifier_time_l0.as_secs_f32() / n_transactions as f32
    );

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
        let metadata = metadata(&TMP_PATH).unwrap();
        total_size += metadata.size();
    }

    let total_size = total_size as f32;

    log::info!(" - Number of transactions: {:?}", n_transactions);
    log::info!(" - Total proof size: {:?} MB", total_size / 1_000_000_f32);
    log::info!(
        "   Average proof size: {:?} MB",
        total_size / ((n_transactions * 1_000_000) as f32)
    );
    log::info!(" - Total prover time: {:?}", total_prover_time_l1);
    log::info!(
        "   Average prover time: {:?} s.",
        total_prover_time_l1.as_secs_f32() / n_transactions as f32
    );
    log::info!(" - Total verifier time: {:?}", total_verifier_time_l1);
    log::info!(
        "   Average verifier time: {:?} s.",
        total_verifier_time_l1.as_secs_f32() / n_transactions as f32
    );

    // Third measurement: Aggregate plonky2 proofs in pairs
    log::info!("\n\n******** Level 2: Two levels of recursion ********");

    log::info!("Starky config:\n{:?}", STARKY_PROVER_CONFIG);

    let timer = std::time::Instant::now();

    let aggregated_proofs = root_proofs
        .into_iter()
        .chunks(2)
        .into_iter()
        .map(|c| {
            let ((proof_0, pv_0), (proof_1, pv_1)) = c.into_iter().collect_tuple().unwrap();
            prover_state
                .prove_aggregation(false, &proof_0, pv_0, false, &proof_1, pv_1)
                .unwrap()
        })
        .collect::<Vec<_>>();

    let total_prover_time_l2 = timer.elapsed() + total_prover_time_l1;

    let timer = std::time::Instant::now();
    for proof in aggregated_proofs.iter() {
        prover_state.verify_aggregation(&proof.0).unwrap();
    }
    let total_verifier_time_l2 = timer.elapsed();

    let mut total_size = 0;

    for proof in aggregated_proofs.iter() {
        serde_json::to_writer(std::fs::File::create(&TMP_PATH).unwrap(), &proof.0).unwrap();
        let metadata = metadata(&TMP_PATH).unwrap();
        total_size += metadata.size();
    }
    let total_size = total_size as f32;

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

    log::info!("\n\n******** Finished ********");
}
