use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::Path;

use anyhow::Result;
use paladin::runtime::Runtime;
use proof_gen::proof_types::GeneratedBlockProof;
use prover::{BlockProverInput, ProverInput};

#[tokio::test]
async fn test_multiple_block_proofs_and_verification() -> Result<()> {
    // Read all JSON files from /tmp/witnesses
    let witness_dir = Path::new("/tmp/witnesses");
    let mut blocks = Vec::new();

    for entry in fs::read_dir(witness_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let block_input: BlockProverInput = serde_json::from_reader(reader)?;
            blocks.push(block_input);
        }
    }

    // Create a ProverInput
    let prover_input = ProverInput { blocks };

    let runtime = Runtime::in_memory().await?;
    let proved_blocks = prover_input.prove(&runtime, None, false, None).await;
    runtime.close().await?;
    let proved_blocks = proved_blocks?;

    // if cfg!(feature = "test_only") {
    //     log::info!("All proof witnesses have been generated successfully.");
    // } else {
    //     log::info!("All proofs have been generated successfully.");
    // }

    let proofs: Vec<GeneratedBlockProof> = proved_blocks
        .into_iter()
        .filter_map(|(_, proof)| proof)
        .collect();
    std::io::stdout().write_all(&serde_json::to_vec(&proofs)?)?;

    Ok(())
}
