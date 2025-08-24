//! Implemention of a CTV coinpool for a prediction market.
//!
//! Bets are aggregated into a single UTXO, and the pool is split between the winning bets after the settlement.
//! In case of an oracle failure, the pool allows to withdraw the bets after a timeout.
//!
//! Reuses code from https://github.com/stutxo/op_ctv_payment_pool

use crate::{
    withdraw::{build_withdraw_transaction, WithdrawParams, WithdrawType},
    PredictionMarket,
};
use anyhow::Context;
use bitcoin::OutPoint;
use bitcoin::{
    consensus::Encodable,
    hashes::{sha256, Hash},
    key::Secp256k1,
    opcodes::all::{OP_DROP, OP_NOP4, OP_NOP5},
    script::Builder,
    taproot::TaprootBuilder,
    Address, Opcode, ScriptBuf, Sequence, Transaction, XOnlyPublicKey,
};

/// The Check Template Verify opcode.
pub const OP_CTV: Opcode = OP_NOP4;
/// The Check Signature From Stack opcode.
pub const OP_CSFS: Opcode = OP_NOP5;

/// Generate the pool address for a market.
///
/// The pool address is a Taproot address with the following structure:
/// - Path 0: CSFS verification for outcome A
/// - Path 1: CSFS verification for outcome B
/// - Path 2: Escape (withdrawal) branch
pub fn generate_pool_address(market: &PredictionMarket) -> anyhow::Result<Address> {
    let escape_ctv_hash = calculate_ctv_hash_for_escape_tx(market)?;
    let escape_script = build_script_for_escape(escape_ctv_hash);

    let outcome_a_ctv_hash = calculate_ctv_hash_for_payout_tx(market, 'A')?;
    let outcome_a_script = build_script_for_outcome(
        outcome_a_ctv_hash,
        &market.oracle_pubkey,
        &market.outcome_a.nostr_id(),
    )?;

    let outcome_b_ctv_hash = calculate_ctv_hash_for_payout_tx(market, 'B')?;
    let outcome_b_script = build_script_for_outcome(
        outcome_b_ctv_hash,
        &market.oracle_pubkey,
        &market.outcome_b.nostr_id(),
    )?;

    let nums_point = PredictionMarket::nums_point()?;
    let secp = Secp256k1::new();

    let spend_info = TaprootBuilder::new()
        .add_leaf(2, outcome_a_script)?
        .add_leaf(2, outcome_b_script)?
        .add_leaf(1, escape_script)?
        .finalize(&secp, nums_point)
        .map_err(|e| anyhow::anyhow!("Failed to finalize taproot: {e:?}"))?;

    let address = Address::p2tr_tweaked(spend_info.output_key(), market.network);
    Ok(address)
}

/// Build the script for a successful (payout based on the winning outcome) branch.
pub fn build_script_for_outcome(
    ctv_hash: [u8; 32],
    oracle_pubkey: &str,
    outcome: &str,
) -> anyhow::Result<ScriptBuf> {
    let outcome_hash = sha256::Hash::hash(outcome.as_bytes());

    let oracle_pubkey_bytes = hex::decode(oracle_pubkey)
        .with_context(|| format!("Failed to decode oracle pubkey hex: {}", oracle_pubkey))?;
    let oracle_pubkey = XOnlyPublicKey::from_slice(&oracle_pubkey_bytes).with_context(|| {
        format!(
            "Invalid oracle pubkey bytes: {}",
            hex::encode(&oracle_pubkey_bytes)
        )
    })?;

    let script = Builder::new()
        .push_slice(outcome_hash.as_byte_array())
        .push_x_only_key(&oracle_pubkey)
        .push_opcode(OP_CSFS)
        .push_opcode(OP_DROP)
        .push_slice(ctv_hash)
        .push_opcode(OP_CTV)
        .into_script();
    Ok(script)
}

/// Build the script for an escape (withdrawal) branch.
pub fn build_script_for_escape(ctv_hash: [u8; 32]) -> ScriptBuf {
    Builder::new()
        .push_slice(ctv_hash)
        .push_opcode(OP_CTV)
        .into_script()
}

/// Calculate the CTV hash for a payout to the winning bets.
/// Builds a complete transaction to ensure consistency with withdraw.rs
pub fn calculate_ctv_hash_for_payout_tx(
    market: &PredictionMarket,
    winning_outcome: char,
) -> anyhow::Result<[u8; 32]> {
    // Create a market copy with the winning outcome set for CTV calculation
    let mut market_copy = market.clone();
    market_copy.settled = true;
    market_copy.winning_outcome = Some(winning_outcome);

    // Create withdraw params for a payout transaction
    let params = WithdrawParams {
        market: market_copy,
        withdraw_type: WithdrawType::Payout,
        pool_utxo: OutPoint::null(), // Dummy UTXO for CTV calculation
        fee_rate: None,
    };

    // Build the transaction and extract CTV hash
    let tx = build_withdraw_transaction(params)?;
    let hash = calculate_ctv_hash_from_transaction(&tx);
    Ok(hash)
}

/// Calculate the CTV hash for an escape (withdrawal) transaction.
/// Builds a complete transaction to ensure consistency with withdraw.rs
pub fn calculate_ctv_hash_for_escape_tx(market: &PredictionMarket) -> anyhow::Result<[u8; 32]> {
    // Create withdraw params for an escape transaction
    let params = WithdrawParams {
        market: market.clone(),
        withdraw_type: WithdrawType::Escape,
        pool_utxo: OutPoint::null(), // Dummy UTXO for CTV calculation
        fee_rate: None,
    };

    // Build the transaction and extract CTV hash
    let tx = build_withdraw_transaction(params)?;
    let hash = calculate_ctv_hash_from_transaction(&tx);
    Ok(hash)
}

/// Calculate CTV hash from a complete transaction
pub fn calculate_ctv_hash_from_transaction(tx: &Transaction) -> [u8; 32] {
    let mut buffer = Vec::new();
    buffer.extend((tx.version.0 as i32).to_le_bytes()); // version
    buffer.extend(tx.lock_time.to_consensus_u32().to_le_bytes()); // locktime
    buffer.extend(1_u32.to_le_bytes()); // inputs len (always 1 for our case)

    // Calculate sequence hash
    let seq_hash = if tx.input.is_empty() {
        sha256::Hash::hash(&Sequence::ENABLE_RBF_NO_LOCKTIME.0.to_le_bytes())
    } else {
        sha256::Hash::hash(&tx.input[0].sequence.0.to_le_bytes())
    };
    buffer.extend(seq_hash.to_byte_array()); // sequences

    let outputs_len = tx.output.len() as u32;
    buffer.extend(outputs_len.to_le_bytes()); // outputs len

    let mut output_bytes: Vec<u8> = Vec::new();
    for output in &tx.output {
        output.consensus_encode(&mut output_bytes).unwrap();
    }
    buffer.extend(sha256::Hash::hash(&output_bytes).to_byte_array()); // outputs hash

    // We have a single input in our case, which is pooled UTXO
    buffer.extend(0_u32.to_le_bytes()); // inputs index

    let hash = sha256::Hash::hash(&buffer);
    hash.to_byte_array()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_generate_pool_address_success() {
        let market = create_test_market();
        let result = generate_pool_address(&market);

        if let Err(e) = &result {
            println!("Error generating pool address: {}", e);
        }
        assert!(
            result.is_ok(),
            "Pool address generation should succeed: {:?}",
            result
        );

        let address = result.unwrap();
        // Just verify we got a valid address - network validation is handled during generation

        // Verify it's a Taproot address (starts with bcrt1p for regtest)
        let address_str = address.to_string();
        assert!(
            address_str.starts_with("bcrt1p"),
            "Should be a Taproot address on regtest"
        );
    }

    #[test]
    fn test_generate_pool_address_empty_bets() {
        let mut market = create_test_market();
        market.bets_a.clear();
        market.bets_b.clear();
        market.total_amount = 0;

        let result = generate_pool_address(&market);
        assert!(result.is_err(), "Should fail with empty bets");
    }

    #[test]
    fn test_build_script_for_outcome_invalid_oracle_pubkey() {
        let ctv_hash = [0x42; 32];
        let invalid_oracle_pubkey = "invalid_pubkey";
        let outcome = "Team A wins";

        let result = build_script_for_outcome(ctv_hash, invalid_oracle_pubkey, outcome);
        assert!(result.is_err(), "Should fail with invalid oracle pubkey");
    }

    #[test]
    fn test_calculate_ctv_hash_for_payout_tx_empty_bets() {
        // Create a market with no bets
        let market = create_empty_test_market();
        let result = calculate_ctv_hash_for_payout_tx(&market, 'A');
        assert!(result.is_err(), "Should fail with empty winning bets");
    }

    #[test]
    fn test_script_building_with_different_outcomes() {
        let ctv_hash = [0x42; 32];
        let oracle_pubkey = "ee96d4b9c5e16f3b11e33bb27fe39ae7a57daa6b24210de5b39237993742cc0a";

        let outcome_a = "Team A wins";
        let outcome_b = "Team B wins";

        let script_a = build_script_for_outcome(ctv_hash, oracle_pubkey, outcome_a).unwrap();
        let script_b = build_script_for_outcome(ctv_hash, oracle_pubkey, outcome_b).unwrap();

        // Scripts should be different for different outcomes
        assert_ne!(
            script_a, script_b,
            "Different outcomes should produce different scripts"
        );
    }
}
