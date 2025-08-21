//! Implemention of a CTV coinpool for a prediction market.
//!
//! Bets are aggregated into a single UTXO, and the pool is split between the winning bets after the settlement.
//! In case of an oracle failure, the pool allows to withdraw the bets after a timeout.
//!
//! Reuses code from https://github.com/stutxo/op_ctv_payment_pool

use std::str::FromStr;

use anyhow::Context;
use bitcoin::locktime::absolute::LockTime;
use bitcoin::{
    consensus::Encodable,
    hashes::{sha256, Hash},
    key::Secp256k1,
    opcodes::all::{OP_DROP, OP_NOP4, OP_NOP5},
    policy::DUST_RELAY_TX_FEE,
    script::Builder,
    taproot::TaprootBuilder,
    Address, Amount, Network, Opcode, ScriptBuf, Sequence, TxOut, XOnlyPublicKey,
};

use crate::get_tx_version;
use crate::{market::Bet, PredictionMarket, DEFAULT_MARKET_FEE};

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
    let all_bets = market
        .bets_a
        .iter()
        .cloned()
        .chain(market.bets_b.iter().cloned())
        .collect::<Vec<_>>();
    let settlement_timestamp: u32 = market.settlement_timestamp.try_into().unwrap();
    let escape_locktime = LockTime::from_time(settlement_timestamp + market.withdraw_timeout)?;
    let escape_ctv_hash = calculate_ctv_hash_for_escape_tx(
        &all_bets,
        escape_locktime.to_consensus_u32(),
        market.network,
    )?;
    let escape_script = build_script_for_escape(escape_ctv_hash);

    let outcome_a_ctv_hash =
        calculate_ctv_hash_for_payout_tx(&market.bets_a, market.total_amount, market.network)?;
    let outcome_a_script = build_script_for_outcome(
        outcome_a_ctv_hash,
        &market.oracle_pubkey,
        &market.outcome_a.nostr_id(),
    )?;

    let outcome_b_ctv_hash =
        calculate_ctv_hash_for_payout_tx(&market.bets_b, market.total_amount, market.network)?;
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
pub fn calculate_ctv_hash_for_payout_tx(
    winning_bets: &[Bet],
    pool_size: u64,
    network: Network,
) -> anyhow::Result<[u8; 32]> {
    if winning_bets.is_empty() {
        return Err(anyhow::anyhow!("No winning bets"));
    }

    let pool_after_fees = pool_size.saturating_sub(DEFAULT_MARKET_FEE);
    let winning_side_total = winning_bets.iter().map(|bet| bet.amount).sum::<u64>();
    assert!(
        winning_side_total > 0,
        "Total amount of winning bets must be greater than 0"
    );

    let mut outputs = Vec::with_capacity(winning_bets.len());
    for bet in winning_bets {
        let address = Address::from_str(&bet.payout_address)
            .with_context(|| format!("Failed to parse payout address: {}", bet.payout_address))?
            .require_network(network)
            .with_context(|| {
                format!(
                    "Address {} is not valid for network {:?}",
                    bet.payout_address, network
                )
            })?;
        let amount = (bet.amount * pool_after_fees) / winning_side_total;
        if amount > DUST_RELAY_TX_FEE.into() {
            outputs.push(TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: address.script_pubkey(),
            });
        }
    }

    let hash = calculate_ctv_hash(&outputs, None, network);
    Ok(hash)
}

/// Calculate the CTV hash for an escape (withdrawal) transaction.
pub fn calculate_ctv_hash_for_escape_tx(
    all_bets: &[Bet],
    locktime: u32,
    network: Network,
) -> anyhow::Result<[u8; 32]> {
    let mut outputs = Vec::with_capacity(all_bets.len());
    for bet in all_bets {
        let address = Address::from_str(&bet.payout_address)
            .with_context(|| {
                format!(
                    "Failed to parse escape payout address: {}",
                    bet.payout_address
                )
            })?
            .require_network(network)
            .with_context(|| {
                format!(
                    "Escape address {} is not valid for network {:?}",
                    bet.payout_address, network
                )
            })?;
        outputs.push(TxOut {
            value: Amount::from_sat(bet.amount),
            script_pubkey: address.script_pubkey(),
        });
    }
    let hash = calculate_ctv_hash(&outputs, Some(locktime), network);
    Ok(hash)
}

/// Calculate the default CTV hash of a payout transaction template.
pub fn calculate_ctv_hash(outputs: &[TxOut], locktime: Option<u32>, network: Network) -> [u8; 32] {
    let mut buffer = Vec::new();
    buffer.extend(get_tx_version(network).to_le_bytes()); // version
    buffer.extend(0_i32.to_le_bytes()); // locktime
    buffer.extend(1_u32.to_le_bytes()); // inputs len

    let seq = if let Some(locktime) = locktime {
        sha256::Hash::hash(&Sequence(locktime).0.to_le_bytes())
    } else {
        sha256::Hash::hash(&Sequence::ENABLE_RBF_NO_LOCKTIME.0.to_le_bytes())
    };
    buffer.extend(seq.to_byte_array()); // sequences

    let outputs_len = outputs.len() as u32;
    buffer.extend(outputs_len.to_le_bytes()); // outputs len

    let mut output_bytes: Vec<u8> = Vec::new();
    for output in outputs {
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
    use crate::market::{Bet, PredictionMarket, PredictionOutcome};
    use bitcoin::secp256k1::{Secp256k1, SecretKey};
    use bitcoin::{CompressedPublicKey, Network, PrivateKey};

    fn create_valid_regtest_address(index: u8) -> String {
        let secp = Secp256k1::new();
        // Create a deterministic private key for testing
        let mut secret_bytes = [0u8; 32];
        secret_bytes[0] = index;
        secret_bytes[31] = index;
        let secret_key = SecretKey::from_slice(&secret_bytes).unwrap();
        let private_key = PrivateKey::new(secret_key, Network::Regtest);
        let public_key = CompressedPublicKey::from_private_key(&secp, &private_key).unwrap();
        let address = Address::p2wpkh(&public_key, Network::Regtest);
        address.to_string()
    }

    fn create_test_market() -> PredictionMarket {
        let outcome_a = PredictionOutcome::new(
            "Team A wins".to_string(),
            "ee96d4b9c5e16f3b11e33bb27fe39ae7a57daa6b24210de5b39237993742cc0a".to_string(),
            1735689600,
            'A',
        )
        .unwrap();

        let outcome_b = PredictionOutcome::new(
            "Team B wins".to_string(),
            "ee96d4b9c5e16f3b11e33bb27fe39ae7a57daa6b24210de5b39237993742cc0a".to_string(),
            1735689600,
            'B',
        )
        .unwrap();

        PredictionMarket {
            market_id: "test_market_id".to_string(),
            question: "Who will win the match?".to_string(),
            outcome_a,
            outcome_b,
            oracle_pubkey: "ee96d4b9c5e16f3b11e33bb27fe39ae7a57daa6b24210de5b39237993742cc0a"
                .to_string(),
            settlement_timestamp: 1735689600,
            network: Network::Regtest,
            market_utxo: None,
            total_amount: 300000, // 3 BTC worth in sats
            bets_a: vec![
                Bet {
                    payout_address: create_valid_regtest_address(1),
                    amount: 100000,
                    txid: "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                        .to_string(),
                    vout: 0,
                },
                Bet {
                    payout_address: create_valid_regtest_address(2),
                    amount: 50000,
                    txid: "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabce"
                        .to_string(),
                    vout: 1,
                },
            ],
            bets_b: vec![Bet {
                payout_address: create_valid_regtest_address(3),
                amount: 150000,
                txid: "fedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafed".to_string(),
                vout: 0,
            }],
            settled: false,
            winning_outcome: None,
            withdraw_timeout: 86400, // 1 day
        }
    }

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
        let winning_bets = vec![];
        let pool_size = 300000;
        let network = Network::Bitcoin;

        let result = calculate_ctv_hash_for_payout_tx(&winning_bets, pool_size, network);
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
