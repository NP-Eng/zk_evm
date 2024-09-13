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
    // Load the block input from JSON
    let file = File::open("../../trace_decoder/benches/block_input.json")?;
    let reader = BufReader::new(file);
    let block_input: BlockProverInput = serde_json::from_reader(reader)?;

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

    // Set up the prover
    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();
    let prover_state = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[16..25, 9..20, 12..25, 14..25, 9..20, 12..20, 17..30],
        &config,
    );

    // Prove and verify each transaction
    let mut proofs = Vec::new();
    let mut public_values = Vec::new();

    for input in generation_inputs {
        let (proof, pv) =
            prover_state.prove_root(&all_stark, &config, input, &mut Default::default(), None)?;
        prover_state.verify_root(proof.clone())?;
        proofs.push(proof);
        public_values.push(pv);
    }

    // Aggregate proofs if there's more than one
    let (final_proof, final_pv) = if proofs.len() > 1 {
        let mut current_proof = proofs[0].clone();
        let mut current_pv = public_values[0].clone();

        for (proof, pv) in proofs.iter().zip(public_values.iter()).skip(1) {
            let (agg_proof, agg_pv) = prover_state.prove_aggregation(
                false,
                &current_proof,
                current_pv,
                false,
                proof,
                pv.clone(),
            )?;
            current_proof = agg_proof;
            current_pv = agg_pv;
        }

        (current_proof, current_pv)
    } else {
        (proofs[0].clone(), public_values[0].clone())
    };

    // Verify the final proof
    prover_state.verify_aggregation(&final_proof)?;

    // Generate and verify the block proof
    let (block_proof, _) = prover_state.prove_block(None, &final_proof, final_pv)?;
    prover_state.verify_block(&block_proof)?;

    Ok(())
}
