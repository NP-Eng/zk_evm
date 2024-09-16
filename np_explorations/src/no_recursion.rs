use std::{
    env,
    fs::{metadata, File},
    io::BufReader,
    os::unix::fs::MetadataExt,
    time::Duration,
};

use evm_arithmetization::{
    prover::prove, testing_utils::init_logger, verifier::verify_proof, AllStark, GenerationInputs,
    StarkConfig,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    plonk::config::{KeccakGoldilocksConfig, PoseidonGoldilocksConfig},
    timed,
    util::timing::TimingTree,
};
use prover::BlockProverInput;
use trace_decoder::entrypoint;

#[cfg(test)]
mod exploration_1;
#[cfg(test)]
mod exploration_2;
#[cfg(test)]
mod exploration_3;

fn get_generation_inputs_from_json() -> Vec<GenerationInputs> {
    // Load the block input from JSON
    let file = File::open("np_explorations/data/bench_1/block_input.json").unwrap();
    let reader = BufReader::new(file);
    let block_input: BlockProverInput = serde_json::from_reader(reader).unwrap();

    log::info!("Loaded block input from JSON");

    // Create a BlockProverInput
    let block_prover_input = BlockProverInput {
        block_trace: block_input.block_trace,
        other_data: block_input.other_data,
    };

    // Use trace decoder to parse into GenerationInputs
    entrypoint(
        block_prover_input.block_trace.clone(),
        block_prover_input.other_data.clone(),
        |_| unimplemented!("Code hash resolution not implemented for this test"),
    )
    .expect("Failed to get generation inputs from JSON")
}

const D: usize = 2;
type F = GoldilocksField;

// Poseidon config
type PC = PoseidonGoldilocksConfig;

// Keccack config
type KC = KeccakGoldilocksConfig;

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
        "fri_verifier" => FriConfig {
            rate_bits: 4,
            cap_height: 4,
            proof_of_work_bits: 16,
            reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
            num_query_rounds: 21,
        },
        s @ _ => {
            panic!("Invalid FRI config '{s}'. It should be one of {{fri_prover, fri_verifier}}")
        }
    };

    let inputs = get_generation_inputs_from_json();

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
