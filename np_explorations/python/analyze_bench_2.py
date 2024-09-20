import re
import pandas as pd

# Function to parse the data from the file content
def parse_data(file_content):
    level = None
    transactions = []
    current_transaction = {}

    for line in file_content:
        # Detect recursion level
        if "Level 0: No recursion" in line:
            level = "no_recursion"
        elif "Level 1: One level of recursion" in line:
            level = "one_recursion"

        # Detect proving time for no recursion
        if level == "no_recursion" and "Tx" in line and "proved in" in line:
            tx_info = re.findall(r"Tx (\d+) proved in (\d+\.\d+)s", line)
            if tx_info:
                tx_root, proving_time = int(tx_info[0][0]), float(tx_info[0][1])
                if tx_root not in current_transaction:
                    current_transaction[tx_root] = {"no_recursion": None, "one_recursion": None}
                current_transaction[tx_root]["no_recursion"] = proving_time

        # Detect proving time for one recursion
        if level == "one_recursion" and "Tx root" in line and "proved in" in line:
            tx_info = re.findall(r"Tx root (\d+) proved in (\d+\.\d+)s", line)
            if tx_info:
                tx_root, proving_time = int(tx_info[0][0]), float(tx_info[0][1])
                if tx_root not in current_transaction:
                    current_transaction[tx_root] = {"no_recursion": None, "one_recursion": None}
                current_transaction[tx_root]["one_recursion"] = proving_time

    # Convert to list of transactions
    for tx_root, times in current_transaction.items():
        if times["no_recursion"] is not None and times["one_recursion"] is not None:
            transactions.append({
                "tx_root": tx_root,
                "no_recursion_time": times["no_recursion"],
                "one_recursion_time": times["one_recursion"],
                "time_difference": times["one_recursion"] - times["no_recursion"]
            })
    
    return pd.DataFrame(transactions)

with open("../zk_evm_benches/bench_2/large_bench_2_combined.log", "r") as file:
    file_content = file.readlines()

# Parse the data
df = parse_data(file_content)

# Display the results
print("Analysis of proving time difference between no recursion and 1 level of recursion:")
print(df)

# Optionally save the results to a CSV file
# df.to_csv("proving_time_comparison.csv", index=False)
