use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use ethereum_types::{Address, BigEndianHash, H256, U256};
use evm_arithmetization::generation::mpt::{AccountRlp, LegacyReceiptRlp};
use evm_arithmetization::generation::{GenerationInputs, TrieInputs};
use evm_arithmetization::proof::{BlockHashes, BlockMetadata, TrieRoots};
use evm_arithmetization::prover::prove;
use evm_arithmetization::testing_utils::{
    beacon_roots_account_nibbles, beacon_roots_contract_from_storage, eth_to_wei,
    ger_account_nibbles, init_logger, preinitialized_state_and_storage_tries,
    update_beacon_roots_account_storage, GLOBAL_EXIT_ROOT_ACCOUNT,
};
use evm_arithmetization::verifier::verify_proof;
use evm_arithmetization::{AllStark, Node, StarkConfig};
use hex_literal::hex;
use keccak_hash::keccak;
use mpt_trie::nibbles::Nibbles;
use mpt_trie::partial_trie::{HashedPartialTrie, PartialTrie};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::KeccakGoldilocksConfig;
use plonky2::timed;
use plonky2::util::timing::TimingTree;

type F = GoldilocksField;
const D: usize = 2;
type C = KeccakGoldilocksConfig;

/// Test a simple token transfer to a new address.
#[test]
fn test_simple_transfer_starks() -> anyhow::Result<()> {
    let produce_proof = true;

    let output = if produce_proof {
        init_logger();

        let all_stark = AllStark::<F, D>::default();
        let config = StarkConfig::standard_fast_config();

        let beneficiary = hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
        let sender = hex!("2c7536e3605d9c16a7a3d7b1898e529396a65c23");
        let to = hex!("a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0");

        let sender_state_key = keccak(sender);
        let to_state_key = keccak(to);

        let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
        let to_nibbles = Nibbles::from_bytes_be(to_state_key.as_bytes()).unwrap();

        let sender_account_before = AccountRlp {
            nonce: 5.into(),
            balance: eth_to_wei(100_000.into()),
            storage_root: HashedPartialTrie::from(Node::Empty).hash(),
            code_hash: keccak([]),
        };
        let to_account_before = AccountRlp::default();

        let (mut state_trie_before, storage_tries) = preinitialized_state_and_storage_tries()?;

        let mut beacon_roots_account_storage = storage_tries[0].1.clone();
        state_trie_before.insert(sender_nibbles, rlp::encode(&sender_account_before).to_vec())?;

        let tries_before = TrieInputs {
            state_trie: state_trie_before,
            transactions_trie: HashedPartialTrie::from(Node::Empty),
            receipts_trie: HashedPartialTrie::from(Node::Empty),
            storage_tries,
        };

        // Generated using a little py-evm script.
        let txn = hex!("f861050a8255f094a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0648242421ba02c89eb757d9deeb1f5b3859a9d4d679951ef610ac47ad4608dc142beb1b7e313a05af7e9fbab825455d36c36c7f4cfcafbeafa9a77bdff936b52afb36d4fe4bcdd");
        let value = U256::from(100u32);

        let block_metadata = BlockMetadata {
            block_beneficiary: Address::from(beneficiary),
            block_timestamp: 0x03e8.into(),
            block_number: 1.into(),
            block_difficulty: 0x020000.into(),
            block_random: H256::from_uint(&0x020000.into()),
            block_gaslimit: 0xff112233u32.into(),
            block_chain_id: 1.into(),
            block_base_fee: 0xa.into(),
            block_gas_used: 21032.into(),
            ..Default::default()
        };

        let mut contract_code = HashMap::new();
        contract_code.insert(keccak(vec![]), vec![]);

        let expected_state_trie_after: HashedPartialTrie = {
            let mut state_trie_after = HashedPartialTrie::from(Node::Empty);

            let txdata_gas = 2 * 16;
            let gas_used = 21_000 + txdata_gas;

            update_beacon_roots_account_storage(
                &mut beacon_roots_account_storage,
                block_metadata.block_timestamp,
                block_metadata.parent_beacon_block_root,
            )?;
            let beacon_roots_account =
                beacon_roots_contract_from_storage(&beacon_roots_account_storage);

            // TODO: why gas_used * 10?

            let sender_account_after = AccountRlp {
                balance: sender_account_before.balance - value - gas_used * 10,
                nonce: sender_account_before.nonce + 1,
                ..sender_account_before
            };
            let to_account_after = AccountRlp {
                balance: value,
                ..to_account_before
            };

            state_trie_after.insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec())?;
            state_trie_after.insert(to_nibbles, rlp::encode(&to_account_after).to_vec())?;

            state_trie_after.insert(
                beacon_roots_account_nibbles(),
                rlp::encode(&beacon_roots_account).to_vec(),
            )?;
            state_trie_after.insert(
                ger_account_nibbles(),
                rlp::encode(&GLOBAL_EXIT_ROOT_ACCOUNT).to_vec(),
            )?;

            state_trie_after
        };

        let receipt_0 = LegacyReceiptRlp {
            status: true,
            cum_gas_used: 21032.into(),
            bloom: vec![0; 256].into(),
            logs: vec![],
        };
        let mut receipts_trie = HashedPartialTrie::from(Node::Empty);
        receipts_trie.insert(
            // TODO remove
            // This is the same as Nibbles::from_bytes_be(&rlp::NULL_RLP).unwrap(),
            Nibbles::from_str("0x80").unwrap(),
            rlp::encode(&receipt_0).to_vec(),
        )?;
        let transactions_trie: HashedPartialTrie = Node::Leaf {
            nibbles: Nibbles::from_str("0x80").unwrap(),
            value: txn.to_vec(),
        }
        .into();

        let trie_roots_after = TrieRoots {
            state_root: expected_state_trie_after.hash(),
            transactions_root: transactions_trie.hash(),
            receipts_root: receipts_trie.hash(),
        };
        let inputs = GenerationInputs {
            signed_txn: Some(txn.to_vec()),
            withdrawals: vec![],
            global_exit_roots: vec![],
            tries: tries_before,
            trie_roots_after,
            contract_code,
            checkpoint_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
            block_metadata,
            txn_number_before: 0.into(),
            gas_used_before: 0.into(),
            gas_used_after: 21032.into(),
            block_hashes: BlockHashes {
                prev_hashes: vec![H256::default(); 256],
                cur_hash: H256::default(),
            },
        };

        let mut timing = TimingTree::new("prove", log::Level::Info);
        let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing, None)?;
        timing.filter(Duration::from_millis(100)).print();

        // Serializing proof
        serde_json::to_writer(
            std::fs::File::create("data/exploration_2/simple_transfer_proof.json")?,
            &proof,
        )?;

        let mut timing_verify = TimingTree::new("verify", log::Level::Info);
        let output = timed!(
            timing_verify,
            "Verification time",
            verify_proof(&all_stark, proof, &config)
        );
        timing_verify.filter(Duration::from_millis(100)).print();

        output
    } else {
        Ok(())
    };

    output
}

#[test]
fn test_add_11_starks() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
    let sender = hex!("a94f5374fce5edbc8e2a8697c15331677e6ebf0b");
    let to = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender);
    let to_hashed = keccak(to);

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to_nibbles = Nibbles::from_bytes_be(to_hashed.as_bytes()).unwrap();

    let code = [0x60, 0x01, 0x60, 0x01, 0x01, 0x60, 0x00, 0x55, 0x00];
    let code_hash = keccak(code);

    let beneficiary_account_before = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let sender_account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        code_hash,
        ..AccountRlp::default()
    };

    let (mut state_trie_before, mut storage_tries) = preinitialized_state_and_storage_tries()?;
    let mut beacon_roots_account_storage = storage_tries[0].1.clone();
    state_trie_before.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_before).to_vec(),
    )?;
    state_trie_before.insert(sender_nibbles, rlp::encode(&sender_account_before).to_vec())?;
    state_trie_before.insert(to_nibbles, rlp::encode(&to_account_before).to_vec())?;

    storage_tries.push((to_hashed, Node::Empty.into()));

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries,
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

    let txn = hex!("f863800a83061a8094095e7baea6a6c7c4c2dfeb977efac326af552d87830186a0801ba0ffb600e63115a7362e7811894a91d8ba4330e526f22121c994c4692035dfdfd5a06198379fcac8de3dbfac48b165df4bf88e2088f294b61efb9a65fe2281c76e16");

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_random: H256::from_uint(&0x020000.into()),
        block_gaslimit: 0xff112233u32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: 0xa868u64.into(),
        ..Default::default()
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    let expected_state_trie_after = {
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
        let to_account_after = AccountRlp {
            balance: 0xde0b6b3a76586a0u64.into(),
            code_hash,
            // Storage map: { 0 => 2 }
            storage_root: HashedPartialTrie::from(Node::Leaf {
                nibbles: Nibbles::from_h256_be(keccak([0u8; 32])),
                value: vec![2],
            })
            .hash(),
            ..AccountRlp::default()
        };

        let mut expected_state_trie_after = HashedPartialTrie::from(Node::Empty);
        expected_state_trie_after.insert(
            beneficiary_nibbles,
            rlp::encode(&beneficiary_account_after).to_vec(),
        )?;
        expected_state_trie_after
            .insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec())?;
        expected_state_trie_after.insert(to_nibbles, rlp::encode(&to_account_after).to_vec())?;
        expected_state_trie_after.insert(
            beacon_roots_account_nibbles(),
            rlp::encode(&beacon_roots_account).to_vec(),
        )?;
        expected_state_trie_after.insert(
            ger_account_nibbles(),
            rlp::encode(&GLOBAL_EXIT_ROOT_ACCOUNT).to_vec(),
        )?;

        expected_state_trie_after
    };

    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: 0xa868u64.into(),
        bloom: vec![0; 256].into(),
        logs: vec![],
    };
    let mut receipts_trie = HashedPartialTrie::from(Node::Empty);
    receipts_trie.insert(
        Nibbles::from_str("0x80").unwrap(),
        rlp::encode(&receipt_0).to_vec(),
    )?;
    let transactions_trie: HashedPartialTrie = Node::Leaf {
        nibbles: Nibbles::from_str("0x80").unwrap(),
        value: txn.to_vec(),
    }
    .into();

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };
    let inputs = GenerationInputs {
        signed_txn: Some(txn.to_vec()),
        withdrawals: vec![],
        global_exit_roots: vec![],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        block_metadata,
        checkpoint_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: 0xa868u64.into(),
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
    };

    let mut timing = TimingTree::new("prove", log::Level::Info);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing, None)?;
    timing.filter(Duration::from_millis(100)).print();

    // Serializing proof
    serde_json::to_writer(
        std::fs::File::create("data/exploration_2/add_11_proof.json")?,
        &proof,
    )?;

    let mut timing_verify = TimingTree::new("verify", log::Level::Info);
    let output = timed!(
        timing_verify,
        "Verification time",
        verify_proof(&all_stark, proof, &config)
    );
    timing_verify.filter(Duration::from_millis(100)).print();

    output
}