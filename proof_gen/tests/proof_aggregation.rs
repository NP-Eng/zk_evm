#[cfg(test)]
use std::collections::HashMap;
use std::time::Duration;

use ethereum_types::{Address, BigEndianHash, H256, U256};
use evm_arithmetization::generation::mpt::{AccountRlp, LegacyReceiptRlp};
use evm_arithmetization::generation::{GenerationInputs, TrieInputs};
use evm_arithmetization::proof::{BlockHashes, BlockMetadata, PublicValues, TrieRoots};
use evm_arithmetization::testing_utils::{
    beacon_roots_account_nibbles, beacon_roots_contract_from_storage, eth_to_wei,
    ger_account_nibbles, init_logger, preinitialized_state_and_storage_tries,
    update_beacon_roots_account_storage, GLOBAL_EXIT_ROOT_ACCOUNT,
};
use evm_arithmetization::{AllRecursiveCircuits, AllStark, Node, StarkConfig};
use hex_literal::hex;
use keccak_hash::keccak;
use mpt_trie::nibbles::Nibbles;
use mpt_trie::partial_trie::{HashedPartialTrie, PartialTrie};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::timed;
use plonky2::util::timing::TimingTree;

// Following https://docs.rs/evm_arithmetization/latest/evm_arithmetization/

// Specify the base field to use.
type F = GoldilocksField;
// Specify the extension degree to use.
const D: usize = 2;
// Specify the recursive configuration to use, here leveraging Poseidon hash
// over the Goldilocks field both natively and in-circuit.
type C = PoseidonGoldilocksConfig;

#[test]
fn test_proof_aggregation() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    // Creating txn1 accounts
    let beneficiary = hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
    let sender_txn1 = hex!("a94f5374fce5edbc8e2a8697c15331677e6ebf0b");
    let to_txn1 = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_txn1_state_key = keccak(sender_txn1);
    let to_txn1_state_key = keccak(to_txn1);

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_txn1_nibbles = Nibbles::from_bytes_be(sender_txn1_state_key.as_bytes()).unwrap();
    let to_txn1_nibbles = Nibbles::from_bytes_be(to_txn1_state_key.as_bytes()).unwrap();

    let code_txn1 = [0x60, 0x01, 0x60, 0x01, 0x01, 0x60, 0x00, 0x55, 0x00];
    let code_hash_txn1 = keccak(code_txn1);

    let beneficiary_account_before = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let sender_txn1_account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        ..AccountRlp::default()
    };
    let to_txn1_account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        code_hash: code_hash_txn1,
        ..AccountRlp::default()
    };

    let gas_used_txn1: U256 = 0xa868u64.into();

    // Creating txn2 accounts
    let sender_txn2 = hex!("2c7536e3605d9c16a7a3d7b1898e529396a65c23");
    let to_txn2 = hex!("a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0");

    let sender_txn2_state_key = keccak(sender_txn2);
    let to_txn2_state_key = keccak(to_txn2);

    let sender_txn2_nibbles = Nibbles::from_bytes_be(sender_txn2_state_key.as_bytes()).unwrap();
    let to_txn2_nibbles = Nibbles::from_bytes_be(to_txn2_state_key.as_bytes()).unwrap();

    let sender_txn2_account_before = AccountRlp {
        nonce: 5.into(),
        balance: eth_to_wei(100_000.into()),
        storage_root: HashedPartialTrie::from(Node::Empty).hash(),
        code_hash: keccak([]),
    };
    let to_txn2_account_before = AccountRlp::default();

    let txdata_gas_txn2 = 2 * 16;
    let gas_used_txn2: U256 = (21_000 + txdata_gas_txn2).into();

    // Creating world state before txn1
    let (mut state_trie_before_txn1, mut storage_tries_before_txn1) =
        preinitialized_state_and_storage_tries()?;

    let mut beacon_roots_account_storage = storage_tries_before_txn1[0].1.clone();
    state_trie_before_txn1.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_before).to_vec(),
    )?;
    state_trie_before_txn1.insert(
        sender_txn1_nibbles,
        rlp::encode(&sender_txn1_account_before).to_vec(),
    )?;
    state_trie_before_txn1.insert(
        to_txn1_nibbles,
        rlp::encode(&to_txn1_account_before).to_vec(),
    )?;
    state_trie_before_txn1.insert(
        sender_txn2_nibbles,
        rlp::encode(&sender_txn2_account_before).to_vec(),
    )?;

    storage_tries_before_txn1.push((to_txn1_state_key, Node::Empty.into()));

    let checkpoint_hash = state_trie_before_txn1.hash();

    let tries_before_txn1 = TrieInputs {
        state_trie: state_trie_before_txn1.clone(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: storage_tries_before_txn1.clone(),
    };

    /*
       {
           "chainId": "-4",
           "type": "LegacyTransaction",
           "valid": true,
           "nonce": "0",
           "gasPrice": "10" (0a),
           "gasLimit": "400000" (061a80),
           "from": "0xa94f5374Fce5edBC8E2a8697C15331677e6EbF0B",
           "to": "0x095e7baea6a6c7c4c2dfeb977efac326af552d87",
           "value": "100000" (0186a0),

           --> hash of the above = "0xeda4d6763740fbccc99cc8873ff09b8504d192e83f73bd16ccf5feb053a4e3cd",
           --> signature of the hash =
               "v": "1b",
               "r": "ffb600e63115a7362e7811894a91d8ba4330e526f22121c994c4692035dfdfd5",
               "s": "6198379fcac8de3dbfac48b165df4bf88e2088f294b61efb9a65fe2281c76e16",

           Final data: original data + signature of the hash of the original data
       }
    */

    let txn1 = hex!("f863800a83061a8094095e7baea6a6c7c4c2dfeb977efac326af552d87830186a0801ba0ffb600e63115a7362e7811894a91d8ba4330e526f22121c994c4692035dfdfd5a06198379fcac8de3dbfac48b165df4bf88e2088f294b61efb9a65fe2281c76e16");

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_random: H256::from_uint(&0x020000.into()),
        block_gaslimit: 0xff112233u32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: gas_used_txn1 + gas_used_txn2,
        ..Default::default()
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash_txn1, code_txn1.to_vec());

    let (
        state_trie_after_txn1,
        beacon_roots_account_storage_after_txn1,
        to_account_storage_after_txn1,
    ) = {
        update_beacon_roots_account_storage(
            &mut beacon_roots_account_storage,
            block_metadata.block_timestamp,
            block_metadata.parent_beacon_block_root,
        )?;
        let beacon_roots_account =
            beacon_roots_contract_from_storage(&beacon_roots_account_storage);

        let beneficiary_account_after = AccountRlp {
            nonce: 1.into(),
            ..AccountRlp::default()
        };
        let sender_account_after = AccountRlp {
            balance: 0xde0b6b3a75be550u64.into(),
            nonce: 1.into(),
            ..AccountRlp::default()
        };

        let to_account_storage_after_txn1 = HashedPartialTrie::from(Node::Leaf {
            nibbles: Nibbles::from_h256_be(keccak([0u8; 32])),
            value: vec![2],
        });

        let to_account_after = AccountRlp {
            balance: 0xde0b6b3a76586a0u64.into(),
            code_hash: code_hash_txn1,
            // Storage map: { 0 => 2 }
            storage_root: to_account_storage_after_txn1.hash(),
            ..AccountRlp::default()
        };

        let mut state_trie_after_txn1 = HashedPartialTrie::from(Node::Empty);
        state_trie_after_txn1.insert(
            beneficiary_nibbles,
            rlp::encode(&beneficiary_account_after).to_vec(),
        )?;
        state_trie_after_txn1.insert(
            sender_txn1_nibbles,
            rlp::encode(&sender_account_after).to_vec(),
        )?;
        state_trie_after_txn1.insert(to_txn1_nibbles, rlp::encode(&to_account_after).to_vec())?;
        state_trie_after_txn1.insert(
            sender_txn2_nibbles,
            rlp::encode(&sender_txn2_account_before).to_vec(),
        )?;
        state_trie_after_txn1.insert(
            beacon_roots_account_nibbles(),
            rlp::encode(&beacon_roots_account).to_vec(),
        )?;
        state_trie_after_txn1.insert(
            ger_account_nibbles(),
            rlp::encode(&GLOBAL_EXIT_ROOT_ACCOUNT).to_vec(),
        )?;

        (
            state_trie_after_txn1,
            beacon_roots_account_storage,
            to_account_storage_after_txn1,
        )
    };

    // TODO remove
    // println!(
    //     "Before pop-push: Beacons account storage hash in
    // storage_tries_after_txn1: {:?}",     storage_tries_before_txn1[0].1.
    // hash() );

    // Updating storage database

    let mut storage_tries_after_txn1 = Vec::new();

    // Beacon root storage
    storage_tries_after_txn1.push((
        storage_tries_before_txn1[0].0,
        beacon_roots_account_storage_after_txn1,
    ));

    // GER storage (unchanged)
    storage_tries_after_txn1.push(storage_tries_before_txn1[1].clone());

    // to_txn1 storage
    storage_tries_after_txn1.push((to_txn1_state_key, to_account_storage_after_txn1));

    // TODO remove
    // println!(
    //     "After pop-push: Beacons account storage hash in
    // storage_tries_after_txn1: {:?}",     storage_tries_after_txn1[0].1.hash()
    // );

    // TODO remove
    // for (i, storage_pair) in storage_tries_after_txn1.iter().enumerate() {
    //     println!("----\n\tStorage trie {i} key: {:?}", storage_pair.0);
    //     println!("\tStorage trie {i} value: {:?}", storage_pair.1);
    //     println!("\tStorage trie {i}: {:?}", storage_pair.1.hash());
    // }
    // let nibbles = [
    //     (beacon_roots_account_nibbles(), "beacon_roots_account"),
    //     (ger_account_nibbles(), "ger_account"),
    //     (beneficiary_nibbles, "beneficiary"),
    //     (sender_txn1_nibbles, "sender_txn1"),
    //     (to_txn1_nibbles, "to_txn1"),
    //     (sender_txn2_nibbles, "sender_txn2"),
    // ];
    // for (nibs, name) in nibbles.iter() {
    //     println!("- Account: {name}, {:?}", nibs);
    //     let account = state_trie_after_txn1.get(*nibs).unwrap();
    //     let decoded_account = rlp::decode::<AccountRlp>(account).unwrap();
    //     println!("   Storage root: {:?}", decoded_account.storage_root);
    // }

    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: gas_used_txn1,
        bloom: vec![0; 256].into(),
        logs: vec![],
    };
    // let mut receipts_trie_after_txn1 = HashedPartialTrie::from(Node::Empty);
    // receipts_trie_after_txn1.insert(
    //     Nibbles::from_bytes_be(&rlp::encode(&receipt_0).to_vec()[..32]).unwrap(),
    //     rlp::encode(&receipt_0).to_vec(),
    // )?;
    // let transactions_trie_after_txn1: HashedPartialTrie = Node::Leaf {
    //     //nibbles: Nibbles::from_str("0x80").unwrap(),
    //     nibbles: Nibbles::from_bytes_be(&txn1[..32]).unwrap(),
    //     value: txn1.to_vec(),
    // }
    // .into();
    let mut receipts_trie_after_txn1 = HashedPartialTrie::from(Node::Empty);
    receipts_trie_after_txn1.insert(
        // Same as: Nibbles::from_str("0x80").unwrap(),
        Nibbles::from_bytes_be(&rlp::encode(&b'\x00')).unwrap(),
        rlp::encode(&receipt_0).to_vec(),
    )?;
    let transactions_trie_after_txn1: HashedPartialTrie = Node::Leaf {
        // Same as: nibbles: Nibbles::from_str("0x80").unwrap(),
        nibbles: Nibbles::from_bytes_be(&rlp::encode(&b'\x00')).unwrap(),
        value: txn1.to_vec(),
    }
    .into();

    let trie_roots_after_txn1 = TrieRoots {
        state_root: state_trie_after_txn1.hash(),
        transactions_root: transactions_trie_after_txn1.hash(),
        receipts_root: receipts_trie_after_txn1.hash(),
    };

    let inputs_txn1 = GenerationInputs {
        signed_txn: Some(txn1.to_vec()),
        withdrawals: vec![],
        global_exit_roots: vec![],
        tries: tries_before_txn1,
        trie_roots_after: trie_roots_after_txn1,
        contract_code,
        block_metadata: block_metadata.clone(),
        checkpoint_state_trie_root: checkpoint_hash,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: gas_used_txn1,
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
    };

    // let mut timing = TimingTree::new("prove", log::Level::Info);
    // let proof = prove::<F, C, D>(&all_stark, &config, inputs_txn1, &mut timing,
    // None)?; timing.filter(Duration::from_millis(100)).print();

    // let mut timing_verify_txn1 = TimingTree::new("verify", log::Level::Info);
    // timed!(
    //     timing_verify_txn1,
    //     "Verification time",
    //     verify_proof(&all_stark, proof, &config)
    // )?;
    // timing_verify_txn1
    //     .filter(Duration::from_millis(100))
    //     .print();

    /**************************** Second transaction ************************* */

    // Accounts already created above

    // TODO REMOVE
    // println!("state_trie_after_txn1");
    // println!(
    //     "Beacons account storage hash in storage_tries_after_txn1: {:?}",
    //     storage_tries_after_txn1[0].1.hash()
    // );
    // println!(
    //     "Beacons account storage hash in state_trie_after_txn1: {:?}",
    //     rlp::decode::<AccountRlp>(
    //         state_trie_after_txn1
    //             .get(beacon_roots_account_nibbles())
    //             .unwrap()
    //     )
    //     .unwrap()
    //     .storage_root
    // );

    let tries_after_tx1 = TrieInputs {
        state_trie: state_trie_after_txn1.clone(),
        transactions_trie: transactions_trie_after_txn1.clone(),
        receipts_trie: receipts_trie_after_txn1.clone(),
        storage_tries: storage_tries_after_txn1,
    };

    // Generated using a little py-evm script.
    let txn2 = hex!("f861050a8255f094a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0648242421ba02c89eb757d9deeb1f5b3859a9d4d679951ef610ac47ad4608dc142beb1b7e313a05af7e9fbab825455d36c36c7f4cfcafbeafa9a77bdff936b52afb36d4fe4bcdd");
    let value_txn2 = U256::from(100u32);

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);

    let expected_state_trie_after_txn2: HashedPartialTrie = {
        let mut state_trie_after = state_trie_after_txn1.clone();

        // TODO: why gas_used * 10?

        let sender_account_after = AccountRlp {
            balance: sender_txn2_account_before.balance - value_txn2 - gas_used_txn2 * 10,
            nonce: sender_txn2_account_before.nonce + 1,
            ..sender_txn2_account_before
        };
        let to_account_after = AccountRlp {
            balance: value_txn2,
            ..to_txn2_account_before
        };

        state_trie_after.insert(
            sender_txn2_nibbles,
            rlp::encode(&sender_account_after).to_vec(),
        )?;
        state_trie_after.insert(to_txn2_nibbles, rlp::encode(&to_account_after).to_vec())?;

        // TODO has this really changed?
        // state_trie_after.insert(
        //     beacon_roots_account_nibbles(),
        //     rlp::encode(&beacon_roots_account).to_vec(),
        // )?;

        state_trie_after
    };

    let receipt_1 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: gas_used_txn1 + gas_used_txn2,
        bloom: vec![0; 256].into(),
        logs: vec![],
    };

    let mut receipts_trie_after_txn2 = receipts_trie_after_txn1;
    receipts_trie_after_txn2.insert(
        // TODO remove
        // This is the same as Nibbles::from_bytes_be(&rlp::NULL_RLP).unwrap(),
        Nibbles::from_bytes_be(&rlp::encode(&b'\x01')).unwrap(),
        rlp::encode(&receipt_1).to_vec(),
    )?;

    // println!("ENCODING OF 0: {:?}", rlp::encode(&b'\0'));

    let mut transactions_trie_after_txn2 = transactions_trie_after_txn1;
    transactions_trie_after_txn2.insert(
        Nibbles::from_bytes_be(&rlp::encode(&b'\x01')).unwrap(),
        txn2.to_vec(),
    )?;

    let trie_roots_after_txn2 = TrieRoots {
        state_root: expected_state_trie_after_txn2.hash(),
        transactions_root: transactions_trie_after_txn2.hash(),
        receipts_root: receipts_trie_after_txn2.hash(),
    };

    let inputs_txn2 = GenerationInputs {
        signed_txn: Some(txn2.to_vec()),
        withdrawals: vec![],
        global_exit_roots: vec![],
        tries: tries_after_tx1,
        trie_roots_after: trie_roots_after_txn2,
        contract_code,
        checkpoint_state_trie_root: checkpoint_hash,
        block_metadata: block_metadata,
        txn_number_before: 1.into(),
        gas_used_before: gas_used_txn1,
        gas_used_after: gas_used_txn1 + gas_used_txn2,
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
    };

    // let mut timing = TimingTree::new("prove", log::Level::Info);
    // let proof = prove::<F, C, D>(&all_stark, &config, inputs_txn2, &mut timing,
    // None)?; timing.filter(Duration::from_millis(100)).print();

    // let mut timing_verify = TimingTree::new("verify", log::Level::Info);
    // let output = timed!(
    //     timing_verify,
    //     "Verification time",
    //     verify_proof(&all_stark, proof, &config)
    // );
    // timing_verify.filter(Duration::from_millis(100)).print();

    /****************************** Aggregation ***************************** */

    // Generate all the recursive circuits needed to generate succinct proofs
    // for blocks. The ranges correspond to the supported table sizes for
    // each individual STARK component.
    let prover_state = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        // TODO what is this? It is related to the starky machines and they say it should "be large
        // enough for your application"
        &[16..25, 9..20, 12..25, 14..25, 9..20, 12..20, 17..30],
        &config,
    );

    // TODO remove
    println!("[*] ...Initiating proving");

    let mut timing_tree = TimingTree::new("prove", log::Level::Info);

    // Proving individual transactions
    let (proof_0, pv_0) =
        prover_state.prove_root(&all_stark, &config, inputs_txn1, &mut timing_tree, None)?;
    timing_tree.filter(Duration::from_millis(100)).print();

    // TODO remove
    println!("[*] Finished proof 0");

    serde_json::to_writer(std::fs::File::create("../np_data/proof_0.json")?, &proof_0)?;

    let (proof_1, pv_1) =
        prover_state.prove_root(&all_stark, &config, inputs_txn2, &mut timing_tree, None)?;
    timing_tree.filter(Duration::from_millis(100)).print();

    // TODO remove
    println!("[*] Finished proof 1");

    serde_json::to_writer(std::fs::File::create("../np_data/proof_1.json")?, &proof_1)?;

    // First (and only) aggregation layer
    let (agg_proof, pv) = timed!(
        timing_tree,
        "Aggregation time",
        prover_state.prove_aggregation(false, &proof_0, pv_0, false, &proof_1, pv_1)
    )?;

    serde_json::to_writer(
        std::fs::File::create("../np_data/agg_proof.json")?,
        &agg_proof,
    )?;

    // TODO remove
    println!("[*] Finished aggregated proof");

    // Test retrieved public values from the proof public inputs.
    let retrieved_public_values = PublicValues::from_public_inputs(&agg_proof.public_inputs);
    assert_eq!(retrieved_public_values, pv);
    assert_eq!(
        pv.trie_roots_before.state_root,
        pv.extra_block_data.checkpoint_state_trie_root
    );

    // Proving verification of aggregated proof
    let (block_proof, block_public_values) = timed!(
        timing_tree,
        "Block proof time",
        prover_state.prove_block(
            // We don't specify a previous proof, considering block 1 as the new checkpoint.
            None, &agg_proof, pv,
        )
    )?;

    serde_json::to_writer(
        std::fs::File::create("../np_data/block_proof.json")?,
        &block_proof,
    )?;

    // TODO remove
    println!("[*] Finished block proof");

    let pv_block = PublicValues::from_public_inputs(&block_proof.public_inputs);
    assert_eq!(block_public_values, pv_block);

    // TODO remove
    println!("[*] ...Initiating verification");

    let mut timing_tree = TimingTree::new("verify", log::Level::Info);

    timed!(
        timing_tree,
        "First proof verification time",
        prover_state.verify_root(proof_0)?
    );
    timing_tree.filter(Duration::from_millis(100)).print();

    timed!(
        timing_tree,
        "Second proof verification time",
        prover_state.verify_root(proof_1)?
    );

    // TODO remove
    println!("[*] Finished second proof verification");

    timed!(
        timing_tree,
        "Aggregated proof verification time",
        prover_state.verify_aggregation(&agg_proof)?
    );

    // TODO remove
    println!("[*] Finished aggregated proof verification");

    timed!(
        TimingTree::new("verify", log::Level::Info),
        "Block proof verification time",
        prover_state.verify_block(&block_proof)?
    );

    // TODO remove
    println!("[*] Finished block proof verification");

    Ok(())
}
