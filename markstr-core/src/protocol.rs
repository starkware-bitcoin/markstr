//! Markstr protocol implementation
//!
//! Reuses code from https://github.com/stutxo/op_ctv_payment_pool

use std::str::FromStr;

use bitcoin::locktime::absolute::LockTime;
use bitcoin::{
    consensus::Encodable,
    hashes::{sha256, Hash},
    key::Secp256k1,
    opcodes::all::{OP_DROP, OP_NOP4, OP_NOP5},
    policy::DUST_RELAY_TX_FEE,
    script::Builder,
    taproot::{Signature, TaprootBuilder},
    Address, Amount, Network, Opcode, ScriptBuf, Sequence, TxIn, TxOut,
    XOnlyPublicKey,
};

use crate::{market::Bet, PredictionMarket, DEFAULT_MARKET_FEE};

/// The transaction version for regtest.
pub const TX_VERSION: i32 = 3;
/// The Check Template Verify opcode.
pub const OP_CTV: Opcode = OP_NOP4;
/// The Check Signature From Stack opcode.
pub const OP_CSFS: Opcode = OP_NOP5;

#[derive(Clone, Debug)]
pub enum ProtocolMessage {
    Bet(Bet),
    PartialPoolTx(PartialPoolTx),
}

#[derive(Clone, Debug)]
pub struct PartialPoolTx {
    pub input: TxIn,
    pub signature: Signature,
}

/// Create the pool address for a market.
///
/// The pool address is a Taproot address with the following structure:
/// - Path 0: CSFS verification for outcome A
/// - Path 1: CSFS verification for outcome B
/// - Path 2: Escape (withdrawal) branch
pub fn create_pool_address(market: &PredictionMarket) -> anyhow::Result<Address> {
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

    let oracle_pubkey_bytes = hex::decode(oracle_pubkey)?;
    let oracle_pubkey = XOnlyPublicKey::from_slice(&oracle_pubkey_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid oracle pubkey: {e}"))?;

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
    let pool_after_fees = pool_size.saturating_sub(DEFAULT_MARKET_FEE);
    let winning_side_total = winning_bets.iter().map(|bet| bet.amount).sum::<u64>();
    if winning_side_total == 0 {
        return Err(anyhow::anyhow!("No winning bets"));
    }

    let mut outputs = Vec::with_capacity(winning_bets.len());
    for bet in winning_bets {
        let address = Address::from_str(&bet.payout_address)?.require_network(network)?;
        let amount = (bet.amount * pool_after_fees) / winning_side_total;
        if amount > DUST_RELAY_TX_FEE.into() {
            outputs.push(TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: address.script_pubkey(),
            });
        }
    }

    let hash = calculate_ctv_hash(&outputs, None);
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
        let address = Address::from_str(&bet.payout_address)?.require_network(network)?;
        outputs.push(TxOut {
            value: Amount::from_sat(bet.amount),
            script_pubkey: address.script_pubkey(),
        });
    }
    let hash = calculate_ctv_hash(&outputs, Some(locktime));
    Ok(hash)
}

/// Calculate the default CTV hash of a payout transaction template.
pub fn calculate_ctv_hash(outputs: &[TxOut], locktime: Option<u32>) -> [u8; 32] {
    let mut buffer = Vec::new();
    buffer.extend(TX_VERSION.to_le_bytes()); // version
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
