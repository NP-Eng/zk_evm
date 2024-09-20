import re
import pandas as pd
import os

# Function to parse the data
def parse_data(file_content):
    transactions = []
    current_transaction = {}
    transaction_num = None
    
    for line in file_content:
        if "Transaction" in line:
            if current_transaction:
                transactions.append(current_transaction)
            transaction_num = re.findall(r"Transaction (\d+)", line)[0]
            current_transaction = {"transaction": int(transaction_num)}
        
        if "CPU halted after" in line:
            cycles = int(re.findall(r"CPU halted after (\d+) cycles", line)[0])
            current_transaction["cpu_cycles"] = cycles
        
        if "to prove" in line:
            proving_time = float(re.findall(r"(\d+\.\d+)s to prove", line)[0])
            current_transaction["proving_time"] = proving_time
        
        if "to verify" in line:
            verifying_time = float(re.findall(r"(\d+\.\d+)s to verify", line)[0])
            current_transaction["verifying_time"] = verifying_time
        
        if "TraceCheckpoint" in line:
            trace_lengths = re.findall(r"arithmetic_len: (\d+), byte_packing_len: (\d+), cpu_len: (\d+), keccak_len: (\d+), keccak_sponge_len: (\d+), logic_len: (\d+), memory_len: (\d+)", line)
            if trace_lengths:
                current_transaction["arithmetic_len"] = int(trace_lengths[0][0])
                current_transaction["byte_packing_len"] = int(trace_lengths[0][1])
                current_transaction["cpu_len"] = int(trace_lengths[0][2])
                current_transaction["keccak_len"] = int(trace_lengths[0][3])
                current_transaction["keccak_sponge_len"] = int(trace_lengths[0][4])
                current_transaction["logic_len"] = int(trace_lengths[0][5])
                current_transaction["memory_len"] = int(trace_lengths[0][6])
    
    if current_transaction:
        transactions.append(current_transaction)
    
    return pd.DataFrame(transactions)

# Function to get top X transactions based on criteria
def get_top_transactions(df, top_x, criteria):
    return df.nlargest(top_x, criteria)[["transaction"] + criteria]

# Function to analyze a single file
def analyze_file(file_path):
    print(f"\nAnalyzing file: {file_path}")
    
    # Load the data from file
    with open(file_path, "r") as file:
        file_content = file.readlines()

    # Parse the data
    df = parse_data(file_content)

    # Define the criteria for which you want to find top X transactions
    criteria_list = ["proving_time", "verifying_time", "cpu_cycles", "arithmetic_len", "byte_packing_len", "cpu_len", "keccak_len", "keccak_sponge_len", "logic_len", "memory_len"]

    # Get the top X transactions for each criterion
    top_x = 5
    for criterion in criteria_list:
        print(f"Top {top_x} transactions by {criterion}:")
        top_transactions = get_top_transactions(df, top_x, [criterion])
        print(top_transactions)
        print("\n")

# Process all files in the zk_evm_benches/bench_1 folder
folder_path = "../zk_evm_benches/bench_1"
for filename in os.listdir(folder_path):
    if filename.endswith(".log"):
        file_path = os.path.join(folder_path, filename)
        analyze_file(file_path)
