use std::{env, fs::metadata, os::unix::fs::MetadataExt, time::Duration};

use evm_arithmetization::{
    prover::prove, testing_utils::init_logger, verifier::verify_proof, AllStark, StarkConfig,
};
use plonky2::{
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    timed,
    util::timing::TimingTree,
};

mod common;

use common::{get_generation_inputs_from_json, D, F, KC, PC};

fn main() {
    init_logger();

    let args: Vec<String> = env::args().collect();

    let fri_config = match args[1].as_str() {
        "fri_prover" => FriConfig {
            rate_bits: 1,
            cap_height: 4,
            proof_of_work_bits: 16,
            reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
            num_query_rounds: 84,
        },
        "fri_intermediate" => FriConfig {
            rate_bits: 3,
            cap_height: 4,
            proof_of_work_bits: 16,
            reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
            num_query_rounds: 28,
        },
        "fri_verifier" => FriConfig {
            rate_bits: 4,
            cap_height: 4,
            proof_of_work_bits: 16,
            reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
            num_query_rounds: 21,
        },
        s @ _ => {
            panic!("Invalid FRI config '{s}'. It should be one of {{fri_prover, fri_verifier, fri_intermediate}}")
        }
    };

    let inputs = get_generation_inputs_from_json("np_explorations/data/bench_1/block_input.json");

    log::info!("Number of transactions: {}", inputs.len());

    let all_stark = AllStark::default();
    let stark_config = StarkConfig::new(100, 2, fri_config);

    for (i, generation_inputs) in inputs.into_iter().enumerate() {
        println!("\n\n******** Transaction {i} ********");

        let path = format!("np_explorations/data/bench_1/starky_proofs/txn_{i}.json");

        match args[2].as_str() {
            "poseidon" => {
                let mut timing = TimingTree::new("prove", log::Level::Info);

                let proof = prove::<F, PC, D>(
                    &all_stark,
                    &stark_config,
                    generation_inputs,
                    &mut timing,
                    None,
                )
                .unwrap();
                timing.filter(Duration::from_millis(100)).print();

                // Serializing proof
                serde_json::to_writer(std::fs::File::create(&path).unwrap(), &proof).unwrap();

                let metadata = metadata(&path).unwrap();
                println!("Proof size: {:?} KB", metadata.size() / 1000);

                let mut timing_verify = TimingTree::new("verify", log::Level::Info);
                timed!(
                    timing_verify,
                    "Verification time",
                    verify_proof(&all_stark, proof, &stark_config)
                )
                .unwrap();
                timing_verify.filter(Duration::from_millis(100)).print();
            }
            "keccak" => {
                let mut timing = TimingTree::new("prove", log::Level::Info);

                let proof = prove::<F, KC, D>(
                    &all_stark,
                    &stark_config,
                    generation_inputs,
                    &mut timing,
                    None,
                )
                .unwrap();
                timing.filter(Duration::from_millis(100)).print();

                // Serializing proof
                serde_json::to_writer(std::fs::File::create(&path).unwrap(), &proof).unwrap();

                let metadata = metadata(&path).unwrap();
                println!("Proof size: {:?} KB", metadata.size() / 1000);

                let mut timing_verify = TimingTree::new("verify", log::Level::Info);
                timed!(
                    timing_verify,
                    "Verification time",
                    verify_proof(&all_stark, proof, &stark_config)
                )
                .unwrap();
                timing_verify.filter(Duration::from_millis(100)).print();
            }
            s @ _ => panic!("Invalid hash '{s}'. It should be one of {{poseidon, keccak}}"),
        };
    }
}
