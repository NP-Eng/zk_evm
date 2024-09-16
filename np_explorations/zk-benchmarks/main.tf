provider "aws" {
  region = "eu-central-2" # Change to your desired region
}

# resource "aws_key_pair" "deployer" {
#   key_name   = "deployer-key"
#   public_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAW3Dg2MxGzGmpo5vfZt5Cv5JTtVh7jjjd32JvAs8mUs marti@hungrycats.studio"
# }

# resource "aws_security_group" "allow_ssh" {
#   name        = "allow_ssh"
#   description = "Allow SSH inbound traffic"

#   ingress {
#     description = "SSH from anywhere"
#     from_port   = 22
#     to_port     = 22
#     protocol    = "tcp"
#     cidr_blocks = ["0.0.0.0/0"] # Be cautious: this allows SSH from any IP
#   }

#   egress {
#     from_port   = 0
#     to_port     = 0
#     protocol    = "-1"
#     cidr_blocks = ["0.0.0.0/0"]
#   }

#   tags = {
#     Name = "allow_ssh"
#   }
# }

resource "aws_instance" "zk_instance" {
  count         = 1
  ami           = "ami-08076a271deb06518" # Ubuntu Server 24.04 LTS (HVM), SSD Volume Type
  instance_type = "c5.4xlarge"            # Change to your desired instance type
  # key_name               = aws_key_pair.deployer.key_name # Remove quotes
  # vpc_security_group_ids = [aws_security_group.allow_ssh.id]
  // Remove the iam_instance_profile line

  user_data = <<-EOF
              #!/bin/bash
              # Install build-essential and pkg-config without confirmation
              sudo apt-get update
              sudo apt-get install -y build-essential pkg-config libssl-dev
              
              su - ubuntu << 'USEREOF'
              # Install Rust
              curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

              # Install CloudWatch agent
              wget https://s3.amazonaws.com/amazoncloudwatch-agent/ubuntu/amd64/latest/amazon-cloudwatch-agent.deb
              dpkg -i -E ./amazon-cloudwatch-agent.deb

              # Configure CloudWatch agent
              cat <<EOT > /opt/aws/amazon-cloudwatch-agent/bin/config.json
              {
                "logs": {
                  "logs_collected": {
                    "files": {
                      "collect_list": [
                        {
                          "file_path": "/home/ubuntu/benchmark_output_prover_keccak.log",
                          "log_group_name": "/ec2/zk-benchmark",
                          "log_stream_name": "prover_keccak_{instance_id}"
                        },
                        {
                          "file_path": "/home/ubuntu/benchmark_output_prover_poseidon.log",
                          "log_group_name": "/ec2/zk-benchmark",
                          "log_stream_name": "prover_poseidon_{instance_id}"
                        },
                        {
                          "file_path": "/home/ubuntu/benchmark_output_verifier_keccak.log",
                          "log_group_name": "/ec2/zk-benchmark",
                          "log_stream_name": "verifier_keccak_{instance_id}"
                        },
                        {
                          "file_path": "/home/ubuntu/benchmark_output_verifier_poseidon.log",
                          "log_group_name": "/ec2/zk-benchmark",
                          "log_stream_name": "verifier_poseidon_{instance_id}"
                        }
                      ]
                    }
                  }
                }
              }
              EOT

              # Start CloudWatch agent
              /opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl -a fetch-config -m ec2 -s -c file:/opt/aws/amazon-cloudwatch-agent/bin/config.json

              # Run the benchmark
              curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
              source $HOME/.cargo/env

              # Clone and switch to the correct branch
              git clone https://github.com/NP-Eng/zk_evm.git
              cd zk_evm
              git switch cesar/evm-stark-benches

              # Run the benchmark based on instance index and tee output to a file
              case ${count.index} in
                0)
                  RUST_LOG=info cargo run --release --bin bench_1 -- fri_prover keccak 2>&1 | tee /home/ubuntu/benchmark_output_prover_keccak.log
                  ;;
                1)
                  RUST_LOG=info cargo run --release --bin bench_1 -- fri_prover poseidon 2>&1 | tee /home/ubuntu/benchmark_output_prover_poseidon.log
                  ;;
                2)
                  RUST_LOG=info cargo run --release --bin bench_1 -- fri_verifier keccak 2>&1 | tee /home/ubuntu/benchmark_output_verifier_keccak.log
                  ;;
                3)
                  RUST_LOG=info cargo run --release --bin bench_1 -- fri_verifier poseidon 2>&1 | tee /home/ubuntu/benchmark_output_verifier_poseidon.log
                  ;;
              esac
              EOF

  tags = {
    Name = "zk-benchmark-${count.index + 1}"
  }
}
