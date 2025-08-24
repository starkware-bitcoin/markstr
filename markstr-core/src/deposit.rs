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
    transaction::Version,
    Amount, OutPoint, ScriptBuf, Sequence, TapSighashType, Transaction, TxIn, TxOut, Witness,
};

use crate::{get_tx_version, pool::generate_pool_address, Bet, PredictionMarket};

#[derive(Clone, Debug)]
pub enum ProtocolMessage {
    Bet(Bet),
    PartialDepositTx(PartialDepositTx),
}

#[derive(Clone, Debug)]
pub struct PartialDepositTx {
    pub transaction: Transaction,
    pub input_index: usize,
}

/// Creates a partial transaction with one input (from the bet) and one output (to the pool address).
/// This transaction will later be combined with other participants' inputs.
///
/// # Arguments
/// * `market` - The prediction market
/// * `bet` - The bet containing the input UTXO information
/// * `input_index` - The index of the input in the combined pooltransaction
///
/// # Returns
/// A partial transaction ready to be signed with SIGHASH_SINGLE | SIGHASH_ANYONECANPAY
pub fn create_partial_pool_tx(
    market: &PredictionMarket,
    bet: &Bet,
    input_index: usize,
) -> anyhow::Result<PartialDepositTx> {
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
    let output_amount = bet.amount.saturating_sub(market.fees.fee_per_deposit_output);
    let output = TxOut {
        value: Amount::from_sat(output_amount),
        script_pubkey: pool_address.script_pubkey(),
    };

    // Create the partial transaction
    let transaction = Transaction {
        version: Version(get_tx_version(market.network)),
        lock_time: LockTime::from_time(market.settlement_timestamp as u32)?,
        input: vec![input],
        output: vec![output],
    };

    Ok(PartialDepositTx {
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
    partial_tx: &PartialDepositTx,
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
    partial_tx: &mut PartialDepositTx,
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

/// Combines multiple signed partial transactions into a single pool deposit transaction.
///
/// This function takes a vector of partial transactions that have been signed by participants
/// and combines them into a single transaction that deposits all funds into the market pool.
/// Each partial transaction should contain one input (the bet UTXO) and one output (to the pool address).
///
/// The partial transactions can be provided in arbitrary order - they will be sorted by their
/// `input_index` field to ensure proper ordering in the final transaction.
///
/// # Arguments
/// * `partial_transactions` - Vector of signed partial transactions from participants (can be in any order)
///
/// # Returns
/// A combined transaction ready to be broadcast to the Bitcoin network
///
/// # Errors
/// Returns an error if the partial transactions vector is empty or if any partial transaction is invalid
pub fn combine_deposit_transaction(
    mut partial_transactions: Vec<PartialDepositTx>,
) -> anyhow::Result<Transaction> {
    if partial_transactions.is_empty() {
        return Err(anyhow::anyhow!("Cannot combine empty partial transactions"));
    }

    // Sort partial transactions by input_index to ensure correct ordering
    partial_transactions.sort_by_key(|partial_tx| partial_tx.input_index);

    // Use the first transaction as a template for version and lock_time
    let first_tx = &partial_transactions[0].transaction;

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    // Collect all inputs and outputs from partial transactions in sorted order
    for partial_tx in &partial_transactions {
        // Each partial transaction should have exactly one input and one output
        if partial_tx.transaction.input.len() != 1 {
            return Err(anyhow::anyhow!(
                "Partial transaction must have exactly one input, found {}",
                partial_tx.transaction.input.len()
            ));
        }
        if partial_tx.transaction.output.len() != 1 {
            return Err(anyhow::anyhow!(
                "Partial transaction must have exactly one output, found {}",
                partial_tx.transaction.output.len()
            ));
        }

        inputs.push(partial_tx.transaction.input[0].clone());
        outputs.push(partial_tx.transaction.output[0].clone());
    }

    // Create the combined transaction
    let combined_transaction = Transaction {
        version: first_tx.version,
        lock_time: first_tx.lock_time,
        input: inputs,
        output: outputs,
    };

    Ok(combined_transaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{
        absolute::LockTime, transaction::Version, Address, Amount, OutPoint, ScriptBuf, Sequence,
        TxIn, TxOut, Witness,
    };
    use std::str::FromStr;

    fn create_test_partial_tx(
        txid: &str,
        vout: u32,
        amount: u64,
        input_index: usize,
    ) -> PartialDepositTx {
        let input = TxIn {
            previous_output: OutPoint {
                txid: txid.parse().unwrap(),
                vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        };

        let output = TxOut {
            value: Amount::from_sat(amount),
            script_pubkey: Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
                .unwrap()
                .assume_checked()
                .script_pubkey(),
        };

        let transaction = Transaction {
            version: Version(2),
            lock_time: LockTime::from_time(1735689600).unwrap(),
            input: vec![input],
            output: vec![output],
        };

        PartialDepositTx {
            transaction,
            input_index,
        }
    }

    #[test]
    fn test_combine_deposit_transaction_success() {
        let partial_txs = vec![
            create_test_partial_tx(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                0,
                100000,
                0,
            ),
            create_test_partial_tx(
                "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
                1,
                200000,
                1,
            ),
        ];

        let result = combine_deposit_transaction(partial_txs);
        assert!(result.is_ok());

        let combined_tx = result.unwrap();
        assert_eq!(combined_tx.input.len(), 2);
        assert_eq!(combined_tx.output.len(), 2);
        assert_eq!(combined_tx.version, Version(2));
        assert_eq!(
            combined_tx.lock_time,
            LockTime::from_time(1735689600).unwrap()
        );
    }

    #[test]
    fn test_combine_deposit_transaction_empty() {
        let partial_txs = vec![];
        let result = combine_deposit_transaction(partial_txs);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot combine empty partial transactions"));
    }

    #[test]
    fn test_combine_deposit_transaction_respects_input_order() {
        // Create partial transactions in reverse order to test sorting
        let partial_txs = vec![
            create_test_partial_tx(
                "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
                1,
                200000,
                2,
            ), // index 2
            create_test_partial_tx(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                0,
                100000,
                0,
            ), // index 0
            create_test_partial_tx(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                2,
                150000,
                1,
            ), // index 1
        ];

        let result = combine_deposit_transaction(partial_txs);
        assert!(result.is_ok());

        let combined_tx = result.unwrap();
        assert_eq!(combined_tx.input.len(), 3);
        assert_eq!(combined_tx.output.len(), 3);

        // Verify that inputs are ordered correctly by their original input_index
        // First input should be from txid "1234..." (input_index 0)
        assert_eq!(
            combined_tx.input[0].previous_output.txid.to_string(),
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
        assert_eq!(combined_tx.input[0].previous_output.vout, 0);

        // Second input should be from txid "abcd..." (input_index 1)
        assert_eq!(
            combined_tx.input[1].previous_output.txid.to_string(),
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        );
        assert_eq!(combined_tx.input[1].previous_output.vout, 2);

        // Third input should be from txid "fedc..." (input_index 2)
        assert_eq!(
            combined_tx.input[2].previous_output.txid.to_string(),
            "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321"
        );
        assert_eq!(combined_tx.input[2].previous_output.vout, 1);
    }

    #[test]
    fn test_combine_deposit_transaction_invalid_inputs() {
        // Create a partial transaction with multiple inputs (invalid)
        let mut partial_tx = create_test_partial_tx(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            0,
            100000,
            0,
        );
        partial_tx
            .transaction
            .input
            .push(partial_tx.transaction.input[0].clone());

        let partial_txs = vec![partial_tx];
        let result = combine_deposit_transaction(partial_txs);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Partial transaction must have exactly one input"));
    }

    #[test]
    fn test_combine_deposit_transaction_invalid_outputs() {
        // Create a partial transaction with multiple outputs (invalid)
        let mut partial_tx = create_test_partial_tx(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            0,
            100000,
            0,
        );
        partial_tx
            .transaction
            .output
            .push(partial_tx.transaction.output[0].clone());

        let partial_txs = vec![partial_tx];
        let result = combine_deposit_transaction(partial_txs);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Partial transaction must have exactly one output"));
    }
}
