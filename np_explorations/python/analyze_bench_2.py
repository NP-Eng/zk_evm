import re
import pandas as pd

# Function to parse bench_1 format logs (Poseidon and Keccak)
def parse_bench_1(file_content):
    transactions = []
    current_transaction = {}

    for line in file_content:
        # Detect transaction number
        if "Transaction" in line:
            if current_transaction:
                transactions.append(current_transaction)
            transaction_num = re.findall(r"Transaction (\d+)", line)[0]
            current_transaction = {"transaction": int(transaction_num)}

        # Detect CPU cycles
        if "CPU halted after" in line:
            cycles = int(re.findall(r"CPU halted after (\d+) cycles", line)[0])
            current_transaction["cpu_cycles"] = cycles

        # Detect proving time
        if "to prove" in line:
            proving_time = float(re.findall(r"(\d+\.\d+)s to prove", line)[0])
            current_transaction["proving_time"] = proving_time

    if current_transaction:
        transactions.append(current_transaction)

    return pd.DataFrame(transactions)

# Function to parse bench_2 format logs (Level 1)
def parse_bench_2(file_content):
    transactions = []
    transaction_count = -1  # Start at -1 so the first transaction is 0

    for line in file_content:
        # Detect CPU cycles
        if "CPU halted after" in line:
            cycles = int(re.findall(r"CPU halted after (\d+) cycles", line)[0])
            transaction_count += 1
            current_transaction = {"transaction": transaction_count, "cpu_cycles": cycles}

        # Detect proving time (try different patterns)
        proving_time_patterns = [
            r"proved in (\d+\.\d+)s",
            r"Proving time: (\d+\.\d+)s",
            r"Time to prove: (\d+\.\d+)s"
        ]
        
        for pattern in proving_time_patterns:
            match = re.search(pattern, line)
            if match:
                proving_time = float(match.group(1))
                current_transaction["proving_time"] = proving_time
                transactions.append(current_transaction)
                break

    return pd.DataFrame(transactions)

# Function to process all files and align transactions side by side
def process_files(files):
    dfs = {}

    for file_info in files:
        file_name, label, parser = file_info
        with open(file_name, "r") as file:
            file_content = file.readlines()

        df = parser(file_content)
        df = df[["transaction", "proving_time"]]  # Keep only transaction and proving_time
        df = df.rename(columns={"proving_time": f"{label}_proving_time"})
        dfs[label] = df

    # Merge dataframes based on transaction number
    merged_df = dfs["no_recursion_poseidon"]
    for label in ["no_recursion_keccak", "level_1"]:
        merged_df = pd.merge(merged_df, dfs[label][["transaction", f"{label}_proving_time"]], on="transaction", how="inner")

    # Filter out rows where any of the proving time columns is null
    merged_df = merged_df.dropna(subset=["no_recursion_poseidon_proving_time", "no_recursion_keccak_proving_time", "level_1_proving_time"])

    # Add a new column for the difference between level-1 and keccak proving times
    merged_df['time_difference'] = merged_df['level_1_proving_time'] - merged_df['no_recursion_keccak_proving_time']

    # Calculate averages
    avg_difference = merged_df['time_difference'].mean()
    avg_keccak_time = merged_df['no_recursion_keccak_proving_time'].mean()

    # Calculate the ratio as a percentage
    ratio_percentage = (avg_difference / avg_keccak_time) * 100

    # Print the results
    print(f"Average difference (level-1 - keccak): {avg_difference:.2f} seconds")
    print(f"Average keccak proving time: {avg_keccak_time:.2f} seconds")
    print(f"Ratio of difference to keccak proving time: {ratio_percentage:.2f}%")

    return merged_df

# List of files with their respective parser functions and labels
files = [
    ("../zk_evm_benches/bench_1/large_fast_prover_poseidon.log", "no_recursion_poseidon", parse_bench_1),
    ("../zk_evm_benches/bench_1/large_fast_prover_keccak.log", "no_recursion_keccak", parse_bench_1),
    ("../zk_evm_benches/bench_2/large_bench_2.log", "level_1", parse_bench_2)
]

# Process all files and combine results
df = process_files(files)

# Save the result into a CSV file
df.to_csv("proving_cost_analysis_side_by_side.csv", index=False)
