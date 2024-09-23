import re
import csv

def parse_file(filename):
    data = {}
    with open(filename, 'r') as f:
        lines = f.readlines()
    current_tx = None
    tx_block = []
    for line in lines:
        line = line.strip()
        tx_match = re.match(r"\*\*\*\*\*\*\*\* Transaction (\d+) \*\*\*\*\*\*\*\*", line)
        if tx_match:
            if current_tx is not None:
                # Process the previous transaction block
                tx_data = parse_transaction_block(tx_block)
                data[current_tx] = tx_data
            current_tx = int(tx_match.group(1))
            tx_block = []
        else:
            if current_tx is not None:
                tx_block.append(line)
    # Process the last transaction block
    if current_tx is not None:
        tx_data = parse_transaction_block(tx_block)
        data[current_tx] = tx_data
    return data

def parse_transaction_block(lines):
    tx_data = {}
    for line in lines:
        # CPU halted after X cycles
        match = re.search(r"CPU halted after (\d+) cycles", line)
        if match:
            tx_data['cpu cycles'] = int(match.group(1))
        # CPU trace padded to Y cycles
        match = re.search(r"CPU trace padded to (\d+) cycles", line)
        if match:
            tx_data['cpu trace len'] = int(match.group(1))
        # Trace lengths (before padding): TraceCheckpoint { ... }
        match = re.search(r"Trace lengths \(before padding\): TraceCheckpoint \{(.*)\}", line)
        if match:
            trace_info = match.group(1)
            # Split key-value pairs
            pairs = trace_info.split(',')
            for pair in pairs:
                key_value = pair.strip().split(':')
                if len(key_value) == 2:
                    key = key_value[0].strip()
                    value = int(key_value[1].strip())
                    tx_data[key] = value
        # Xs to prove
        match = re.search(r"(\d+\.\d+)s to prove", line)
        if match:
            tx_data['prover_time'] = float(match.group(1))
        # Proof size: X KB
        match = re.search(r"Proof size: (\d+) KB", line)
        if match:
            tx_data['proof size'] = int(match.group(1))
        # Xs to verify
        match = re.search(r"(\d+\.\d+)s to verify", line)
        if match:
            tx_data['verifier_time'] = float(match.group(1))
    return tx_data

def main():
    fast_prover_file = "../zk_evm_benches/bench_1/medium_fast_prover_keccak.log"
    small_proof_file = "../zk_evm_benches/bench_1/medium_small_proof_keccak.log"
    data_fast_prover = parse_file(fast_prover_file)
    data_small_proof = parse_file(small_proof_file)
    # Prepare CSV data
    csv_rows = []
    headers = ['transaction number', 'cpu cycles', 'cpu trace len', 'arithmetic_len', 'byte_packing_len', 'cpu_len', 'keccak_len', 'keccak_sponge_len', 'logic_len', 'memory_len', 'prover_time_fast_prover', 'prover_time_small_proof', 'verifier_time_fast_prover', 'verifier_time_small_proof', 'proof size_fast_prover', 'proof_size_small_proof', 'prover_time_diff', 'verifier_time_diff', 'proof_size_diff']
    for tx_num in sorted(data_fast_prover.keys()):
        tx_data_fast_prover = data_fast_prover.get(tx_num, {})
        tx_data_small_proof = data_small_proof.get(tx_num, {})
        if not tx_data_fast_prover or not tx_data_small_proof or 'prover_time' not in tx_data_fast_prover or 'prover_time' not in tx_data_small_proof:
            continue  # Skip if there are no results for either fast-prover or small-proof, or if prover_time is not available for either case
        row = {}
        row['transaction number'] = tx_num
        # Since CPU cycles, CPU trace len and trace lengths are the same, we can get them from fast prover data
        for key in ['cpu cycles', 'cpu trace len', 'arithmetic_len', 'byte_packing_len', 'cpu_len', 'keccak_len', 'keccak_sponge_len', 'logic_len', 'memory_len']:
            row[key] = tx_data_fast_prover.get(key, '')
        row['prover_time_fast_prover'] = tx_data_fast_prover.get('prover_time', '')
        row['prover_time_small_proof'] = tx_data_small_proof.get('prover_time', '')
        row['verifier_time_fast_prover'] = tx_data_fast_prover.get('verifier_time', '')
        row['verifier_time_small_proof'] = tx_data_small_proof.get('verifier_time', '')
        row['proof size_fast_prover'] = tx_data_fast_prover.get('proof size', '')
        row['proof_size_small_proof'] = tx_data_small_proof.get('proof size', '')
        # Calculate differences
        try:
            row['prover_time_diff'] = float(row['prover_time_fast_prover']) - float(row['prover_time_small_proof'])
        except:
            row['prover_time_diff'] = ''
        try:
            row['verifier_time_diff'] = float(row['verifier_time_fast_prover']) - float(row['verifier_time_small_proof'])
        except:
            row['verifier_time_diff'] = ''
        try:
            row['proof_size_diff'] = int(row['proof size_fast_prover']) - int(row['proof_size_small_proof'])
        except:
            row['proof_size_diff'] = ''
        csv_rows.append(row)
    # Calculate mean differences
    prover_time_diffs = [row['prover_time_diff'] for row in csv_rows if isinstance(row['prover_time_diff'], float)]
    verifier_time_diffs = [row['verifier_time_diff'] for row in csv_rows if isinstance(row['verifier_time_diff'], float)]
    proof_size_diffs = [row['proof_size_diff'] for row in csv_rows if isinstance(row['proof_size_diff'], int)]
    mean_prover_time_diff = sum(prover_time_diffs) / len(prover_time_diffs) if prover_time_diffs else 0
    mean_verifier_time_diff = sum(verifier_time_diffs) / len(verifier_time_diffs) if verifier_time_diffs else 0
    mean_proof_size_diff = sum(proof_size_diffs) / len(proof_size_diffs) if proof_size_diffs else 0

    # Calculate mean prover time for fast prover
    prover_times_fast_prover = [row['prover_time_fast_prover'] for row in csv_rows if isinstance(row['prover_time_fast_prover'], float)]
    mean_prover_time_fast_prover = sum(prover_times_fast_prover) / len(prover_times_fast_prover) if prover_times_fast_prover else 0

    # Calculate the ratio of mean difference of prover times to mean prover time for fast prover
    if mean_prover_time_fast_prover != 0:
        ratio_prover_time_diff = mean_prover_time_diff / mean_prover_time_fast_prover
    else:
        ratio_prover_time_diff = 0

    # Calculate the ratio of mean difference of proof sizes to mean proof size for fast prover
    proof_sizes_fast_prover = [row['proof size_fast_prover'] for row in csv_rows if isinstance(row['proof size_fast_prover'], int)]
    mean_proof_size_fast_prover = sum(proof_sizes_fast_prover) / len(proof_sizes_fast_prover) if proof_sizes_fast_prover else 0

    if mean_proof_size_fast_prover != 0:
        ratio_proof_size_diff = mean_proof_size_diff / mean_proof_size_fast_prover
    else:
        ratio_proof_size_diff = 0

    # Write CSV
    with open('bench_1_output_fri_config.csv', 'w', newline='') as csvfile:
        writer = csv.DictWriter(csvfile, fieldnames=headers)
        writer.writeheader()
        for row in csv_rows:
            writer.writerow(row)
    print("Mean difference of prover times: {:.4f}s".format(mean_prover_time_diff))
    print("Mean difference of verifier times: {:.4f}s".format(mean_verifier_time_diff))
    print("Ratio of mean difference of prover times to mean prover time for fast prover: {:.2f}%".format(ratio_prover_time_diff * 100))
    print("Ratio of mean difference of proof sizes to mean proof size for fast prover: {:.2f}%".format(ratio_proof_size_diff * 100))

if __name__ == "__main__":
    main()
