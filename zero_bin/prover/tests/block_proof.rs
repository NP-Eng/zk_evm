use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use evm_arithmetization::{AllRecursiveCircuits, AllStark, StarkConfig};
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use prover::BlockProverInput;

type F = GoldilocksField;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;

#[test]
fn test_block_proof_and_verification() -> Result<()> {
    env_logger::builder().is_test(true).init();

    log::info!("Starting block proof and verification test");

    // Load the block input from JSON
    let file = File::open("../../trace_decoder/benches/block_input.json")?;
    let reader = BufReader::new(file);
    let block_input: BlockProverInput = serde_json::from_reader(reader)?;

    log::info!("Loaded block input from JSON");

    // Create a BlockProverInput
    let block_prover_input = BlockProverInput {
        block_trace: block_input.block_trace,
        other_data: block_input.other_data,
    };

    // Use trace decoder to parse into GenerationInputs
    let generation_inputs = trace_decoder::entrypoint(
        block_prover_input.block_trace.clone(),
        block_prover_input.other_data.clone(),
        |_| unimplemented!("Code hash resolution not implemented for this test"),
    )?;

    log::info!("Generated {} inputs for proving", generation_inputs.len());

    // Set up the prover
    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();
    let prover_state = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[16..25, 9..20, 12..25, 14..25, 9..20, 12..20, 17..30],
        &config,
    );

    log::info!("Prover initialized");

    // Prove and verify each transaction
    let mut proofs = Vec::new();
    let mut public_values = Vec::new();

    for (i, input) in generation_inputs.iter().enumerate() {
        log::info!("Proving and verifying transaction {}", i + 1);
        let (proof, pv) = prover_state.prove_root(
            &all_stark,
            &config,
            input.clone(),
            &mut Default::default(),
            None,
        )?;
        prover_state.verify_root(proof.clone())?;
        proofs.push(proof);
        public_values.push(pv);
        log::info!("Transaction {} proved and verified", i + 1);
    }

    Ok(())
}
