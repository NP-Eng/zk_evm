provider "aws" {
  region = "us-east-1" # Change to your desired region
}

resource "aws_instance" "zk_instance" {
  count         = 4
  ami           = "ami-0e86e20dae9224db8" # Ubuntu Server 24.04 LTS (HVM), SSD Volume Type
  instance_type = "c5.4xlarge"             # Change to your desired instance type

  tags = {
    Name = "zk-benchmark-${count.index + 1}"
  }

  # User data for initial setup and running the benchmark
  user_data = <<-EOF
              #!/bin/bash
              curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
              source $HOME/.cargo/env
              git clone https://github.com/NP-Eng/zk_evm.git
              cd zk_evm
              git switch evm-stark-benches
              case ${count.index} in
                0)
                  RUST_LOG=info cargo run --release --bin no-recursion -- fri_prover keccak
                  ;;
                1)
                  RUST_LOG=info cargo run --release --bin no-recursion -- fri_prover poseidon
                  ;;
                2)
                  RUST_LOG=info cargo run --release --bin no-recursion -- fri_verifier keccak
                  ;;
                3)
                  RUST_LOG=info cargo run --release --bin no-recursion -- fri_verifier poseidon
                  ;;
              esac
              EOF
}
