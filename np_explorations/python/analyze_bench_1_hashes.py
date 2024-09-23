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
    keccak_file = "../zk_evm_benches/bench_1/medium_fast_prover_keccak.log"
    poseidon_file = "../zk_evm_benches/bench_1/medium_fast_prover_poseidon.log"
    data_keccak = parse_file(keccak_file)
    data_poseidon = parse_file(poseidon_file)
    # Prepare CSV data
    csv_rows = []
    headers = ['transaction number', 'cpu cycles', 'cpu trace len', 'arithmetic_len', 'byte_packing_len', 'cpu_len', 'keccak_len', 'keccak_sponge_len', 'logic_len', 'memory_len', 'prover_time_keccak', 'prover_time_poseidon', 'verifier_time_keccak', 'verifier_time_poseidon', 'proof size_keccak', 'proof_size_poseidon', 'prover_time_diff', 'verifier_time_diff']
    for tx_num in sorted(data_keccak.keys()):
        tx_data_keccak = data_keccak.get(tx_num, {})
        tx_data_poseidon = data_poseidon.get(tx_num, {})
        row = {}
        row['transaction number'] = tx_num
        # Since CPU cycles, CPU trace len and trace lengths are the same, we can get them from keccak data
        for key in ['cpu cycles', 'cpu trace len', 'arithmetic_len', 'byte_packing_len', 'cpu_len', 'keccak_len', 'keccak_sponge_len', 'logic_len', 'memory_len']:
            row[key] = tx_data_keccak.get(key, '')
        row['prover_time_keccak'] = tx_data_keccak.get('prover_time', '')
        row['prover_time_poseidon'] = tx_data_poseidon.get('prover_time', '')
        row['verifier_time_keccak'] = tx_data_keccak.get('verifier_time', '')
        row['verifier_time_poseidon'] = tx_data_poseidon.get('verifier_time', '')
        row['proof size_keccak'] = tx_data_keccak.get('proof size', '')
        row['proof_size_poseidon'] = tx_data_poseidon.get('proof size', '')
        # Calculate differences
        try:
            row['prover_time_diff'] = float(row['prover_time_keccak']) - float(row['prover_time_poseidon'])
        except:
            row['prover_time_diff'] = ''
        try:
            row['verifier_time_diff'] = float(row['verifier_time_keccak']) - float(row['verifier_time_poseidon'])
        except:
            row['verifier_time_diff'] = ''
        csv_rows.append(row)
    # Calculate mean differences
    prover_time_diffs = [row['prover_time_diff'] for row in csv_rows if isinstance(row['prover_time_diff'], float)]
    verifier_time_diffs = [row['verifier_time_diff'] for row in csv_rows if isinstance(row['verifier_time_diff'], float)]
    mean_prover_time_diff = sum(prover_time_diffs) / len(prover_time_diffs) if prover_time_diffs else 0
    mean_verifier_time_diff = sum(verifier_time_diffs) / len(verifier_time_diffs) if verifier_time_diffs else 0

    # Calculate mean prover time for Poseidon
    prover_times_poseidon = [row['prover_time_poseidon'] for row in csv_rows if isinstance(row['prover_time_poseidon'], float)]
    mean_prover_time_poseidon = sum(prover_times_poseidon) / len(prover_times_poseidon) if prover_times_poseidon else 0

    # Calculate the ratio of mean difference of prover times to mean prover time for Poseidon
    if mean_prover_time_poseidon != 0:
        ratio_prover_time_diff = mean_prover_time_diff / mean_prover_time_poseidon
    else:
        ratio_prover_time_diff = 0

    # Write CSV
    with open('bench_1_output_hashes.csv', 'w', newline='') as csvfile:
        writer = csv.DictWriter(csvfile, fieldnames=headers)
        writer.writeheader()
        for row in csv_rows:
            writer.writerow(row)
    print("Mean difference of prover times:", mean_prover_time_diff)
    print("Mean difference of verifier times:", mean_verifier_time_diff)
    print("Ratio of mean difference of prover times to mean prover time for Poseidon: {:.2f}%".format(ratio_prover_time_diff * 100))

if __name__ == "__main__":
    main()
