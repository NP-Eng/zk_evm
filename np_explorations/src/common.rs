use std::{fs::File, io::BufReader};

use evm_arithmetization::{GenerationInputs, StarkConfig};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    fri::{reduction_strategies::FriReductionStrategy, FriConfig},
    plonk::config::{KeccakGoldilocksConfig, PoseidonGoldilocksConfig},
};
use prover::BlockProverInput;
use trace_decoder::entrypoint;

pub(crate) const D: usize = 2;
pub(crate) type F = GoldilocksField;

// Poseidon config
pub(crate) type PC = PoseidonGoldilocksConfig;

// Keccack config
pub(crate) type KC = KeccakGoldilocksConfig;

pub(crate) const FRI_VERIFIER_CONFIG: FriConfig = FriConfig {
    rate_bits: 4,
    cap_height: 4,
    proof_of_work_bits: 16,
    reduction_strategy: FriReductionStrategy::ConstantArityBits(4, 5),
    num_query_rounds: 21,
};

pub(crate) const STARKY_VERIFIER_CONFIG: StarkConfig = StarkConfig {
    security_bits: 100,
    num_challenges: 2,
    fri_config: FRI_VERIFIER_CONFIG,
};

pub(crate) const STARKY_PROVER_CONFIG: StarkConfig = StarkConfig::standard_fast_config();
// const FRI_PROVER_CONFIG: FriConfig = STARKY_PROVER_CONFIG.fri_config;

pub(crate) fn get_generation_inputs_from_json(json_path: &str) -> Vec<GenerationInputs> {
    // Load the block input from JSON
    let file = File::open(json_path).unwrap();
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
