provider "aws" {
  region = "us-east-1" # Change to your desired region
}

resource "aws_instance" "zk_instance" {
  count         = 4
  ami           = "ami-0e86e20dae9224db8" # Ubuntu Server 24.04 LTS (HVM), SSD Volume Type
  instance_type = "c5.metal"             # Change to your desired instance type

  tags = {
    Name = "zk-benchmark-${count.index + 1}"
  }

  # Optionally, add user data for initial setup
  user_data = <<-EOF
              #!/bin/bash
              curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
              git clone 
              EOF
}
