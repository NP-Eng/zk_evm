# Exploring ZK-EVM proof generation

NP-Labs

## Benchmark 1: no recursion

Test the proof generation time and proof sizes for the following four configs:
- Fast Prover, Keccak
- Fast Prover, Poseidon
- Fast Verifier, Keccak
- Fast Verifier, Poseidon

To run, use: `RUST_LOG=info cargo run --release --bin no-recursion -- A B`

where A is one of `{fri_prover, fri_verifier}` and B is one of `{poseidon, keccak}`