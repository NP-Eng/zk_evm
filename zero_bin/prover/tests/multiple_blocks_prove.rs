use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::Path;

use anyhow::Result;
use paladin::runtime::Runtime;
use proof_gen::proof_types::GeneratedBlockProof;
use prover::{BlockProverInput, ProverInput};

#[tokio::test]
async fn test_multiple_block_proofs_and_verification() -> Result<()> {
    env_logger::builder().is_test(true).init();
    // Read all JSON files from /tmp/witnesses
    let witness_dir = Path::new("/tmp/witnesses");
    let mut blocks = Vec::new();

    log::info!("Starting to read witness files from {:?}", witness_dir);

    let mut entries = fs::read_dir(witness_dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    // Sort entries based on the numeric part of the filename
    entries.sort();

    // Filter for JSON files and process them
    for entry in entries {
        if entry.extension().and_then(|s| s.to_str()) == Some("json") {
            log::info!("Decoding block from file: {:?}", entry);
            let file = File::open(&entry.as_path())?;
            let reader = BufReader::new(file);
            let block_input: Vec<BlockProverInput> = serde_json::from_reader(reader)?;
            blocks.extend(block_input);
            log::info!("Successfully decoded block from {:?}", entry);
        }
    }

    log::info!("Finished reading {} witness files", blocks.len());

    // Create a ProverInput
    let prover_input = ProverInput { blocks };

    log::info!("Starting proof generation");
    let runtime = Runtime::in_memory().await?;
    let proved_blocks = prover_input.prove(&runtime, None, false, None).await;
    runtime.close().await?;
    let proved_blocks = proved_blocks?;

    if cfg!(feature = "test_only") {
        log::info!("All proof witnesses have been generated successfully.");
    } else {
        log::info!("All proofs have been generated successfully.");
    }

    let proofs: Vec<GeneratedBlockProof> = proved_blocks
        .into_iter()
        .filter_map(|(_, proof)| proof)
        .collect();
    log::info!("Writing {} proofs to stdout", proofs.len());
    std::io::stdout().write_all(&serde_json::to_vec(&proofs)?)?;

    Ok(())
}
