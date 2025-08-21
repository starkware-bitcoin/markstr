//! Depositing funds into the pool.
//!
//! The pool is a single UTXO that contains all the bets.
//! Each participant creates and signs a partial transaction with one input (from the bet) and one output (to the pool address).
//! The partial transactions are combined into a single transaction, and submitted to the network.

use bitcoin::{
    absolute::LockTime,
    hashes::Hash,
    key::{Keypair, PrivateKey, Secp256k1},
    secp256k1::Message,
    sighash::{Prevouts, SighashCache},
    taproot::Signature,
    Amount, OutPoint, ScriptBuf, Sequence, TapSighashType, Transaction, TxIn, TxOut, Witness,
};

use crate::{pool::generate_pool_address, Bet, PredictionMarket};

#[derive(Clone, Debug)]
pub enum ProtocolMessage {
    Bet(Bet),
    PartialPoolTx(PartialPoolTx),
}

#[derive(Clone, Debug)]
pub struct PartialPoolTx {
    pub transaction: Transaction,
    pub input_index: usize,
}

/// Creates a partial transaction with one input (from the bet) and one output (to the pool address).
/// This transaction will later be combined with other participants' inputs.
///
/// # Arguments
/// * `market` - The prediction market
/// * `bet` - The bet containing the input UTXO information
/// * `fee_per_input` - Fee to pay for this input (in satoshis)
/// * `input_index` - The index of the input in the combined pooltransaction
///
/// # Returns
/// A partial transaction ready to be signed with SIGHASH_SINGLE | SIGHASH_ANYONECANPAY
pub fn create_partial_pool_tx(
    market: &PredictionMarket,
    bet: &Bet,
    fee_per_input: u64,
    input_index: usize,
) -> anyhow::Result<PartialPoolTx> {
    let pool_address = generate_pool_address(market)?;

    // Create the input from the bet's UTXO
    let input = TxIn {
        previous_output: OutPoint {
            txid: bet.txid.parse()?,
            vout: bet.vout,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
        witness: Witness::new(),
    };

    // Create the output to the pool address
    let output_amount = bet.amount.saturating_sub(fee_per_input);
    let output = TxOut {
        value: Amount::from_sat(output_amount),
        script_pubkey: pool_address.script_pubkey(),
    };

    // Create the partial transaction
    let transaction = Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: LockTime::from_time(market.settlement_timestamp as u32)?,
        input: vec![input],
        output: vec![output],
    };

    Ok(PartialPoolTx {
        transaction,
        input_index,
    })
}

/// Signs a transaction using the provided keypair with SIGHASH_SINGLE | SIGHASH_ANYONECANPAY.
/// This allows the transaction to be combined with other inputs and outputs later.
///
/// # Arguments
/// * `partial_tx` - The partial transaction to sign
/// * `keypair` - The keypair to sign with (can be created from a PrivateKey)
/// * `prevout_value` - The value of the UTXO being spent
/// * `prevout_script` - The script pubkey of the UTXO being spent
///
/// # Returns
/// The signature that can be used in the transaction witness
pub fn sign_partial_transaction(
    partial_tx: &PartialPoolTx,
    keypair: &Keypair,
    prevout_value: u64,
    prevout_script: &ScriptBuf,
) -> anyhow::Result<Signature> {
    let secp = Secp256k1::new();

    // Create the previous outputs for sighash calculation
    let prevouts = vec![TxOut {
        value: Amount::from_sat(prevout_value),
        script_pubkey: prevout_script.clone(),
    }];

    let prevouts = Prevouts::All(&prevouts);

    // Create sighash cache
    let mut sighash_cache = SighashCache::new(&partial_tx.transaction);

    // Use SIGHASH_SINGLE | SIGHASH_ANYONECANPAY to sign only this input and corresponding output
    let sighash_type = TapSighashType::SinglePlusAnyoneCanPay;

    // Calculate the sighash
    let sighash = sighash_cache.taproot_key_spend_signature_hash(
        partial_tx.input_index,
        &prevouts,
        sighash_type,
    )?;

    // Convert to secp256k1 message
    let message = Message::from_digest_slice(sighash.as_byte_array())?;

    // Sign the message
    let signature = secp.sign_schnorr(&message, keypair);

    // Create taproot signature
    let taproot_signature = Signature {
        signature,
        sighash_type,
    };

    Ok(taproot_signature)
}

/// Helper function to create a keypair from a private key
///
/// # Arguments
/// * `private_key` - The private key to convert
///
/// # Returns
/// A keypair that can be used for signing
pub fn keypair_from_private_key(private_key: &PrivateKey) -> anyhow::Result<Keypair> {
    let secp = Secp256k1::new();
    let keypair = Keypair::from_secret_key(&secp, &private_key.inner);
    Ok(keypair)
}

/// Adds a signature to the partial transaction's witness data.
///
/// # Arguments
/// * `partial_tx` - The partial transaction to update
/// * `signature` - The signature to add to the witness
pub fn add_signature_to_partial_tx(
    partial_tx: &mut PartialPoolTx,
    signature: Signature,
) -> anyhow::Result<()> {
    if partial_tx.transaction.input.is_empty() {
        return Err(anyhow::anyhow!("Transaction has no inputs"));
    }

    // Create witness with the signature
    let mut witness = Witness::new();
    witness.push(signature.to_vec());

    // Update the input's witness
    partial_tx.transaction.input[partial_tx.input_index].witness = witness;

    Ok(())
}
