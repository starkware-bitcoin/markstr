//! Withdraw transaction building and signing for prediction markets.
//!
//! This module provides functionality to create and sign withdrawal transactions
//! from prediction market pools. There are two types of withdrawals:
//! 1. Payout transactions - distribute winnings to the winning side after oracle settlement
//! 2. Escape transactions - return funds to all participants after timeout (oracle failure)

use std::str::FromStr;

use anyhow::{Context, Result};
use bitcoin::{
    absolute::LockTime,
    hashes::{sha256, Hash},
    taproot::ControlBlock,
    transaction::Version,
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};

use crate::{
    get_tx_version,
    market::{Bet, MarketFees, PredictionMarket},
    pool::{
        build_script_for_escape, build_script_for_outcome, calculate_ctv_hash_from_transaction,
    },
};

/// Transaction type for withdrawal
#[derive(Debug, Clone, PartialEq)]
pub enum WithdrawType {
    /// Payout to winning bets only (uses market.winning_outcome)
    Payout,
    /// Escape withdrawal returning all funds
    Escape,
}

/// Parameters for building a withdrawal transaction
#[derive(Debug, Clone)]
pub struct WithdrawParams {
    /// The market to withdraw from
    pub market: PredictionMarket,
    /// Type of withdrawal
    pub withdraw_type: WithdrawType,
    /// Pool UTXO to spend
    pub pool_utxo: OutPoint,
    /// Fee rate in sats/vbyte (optional, uses default if None)
    pub fee_rate: Option<u64>,
}

/// Generate transaction outputs for a payout transaction (winning outcome only)
pub fn generate_payout_outputs(
    winning_bets: &[Bet],
    pool_size: u64,
    network: Network,
    fees: &MarketFees,
) -> Result<Vec<TxOut>> {
    if winning_bets.is_empty() {
        return Err(anyhow::anyhow!("No winning bets"));
    }

    let pool_after_fees = fees.pool_after_fees(pool_size, winning_bets.len());
    let winning_side_total = winning_bets.iter().map(|bet| bet.amount).sum::<u64>();

    if winning_side_total == 0 {
        return Err(anyhow::anyhow!(
            "Total amount of winning bets must be greater than 0"
        ));
    }

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
        if amount > 546 {
            // dust threshold
            outputs.push(TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: address.script_pubkey(),
            });
        }
    }
    
    // Add administrator fee output if configured
    if let Some(admin_address) = &fees.administrator_address {
        if fees.administrator_fee > 0 {
            let address = Address::from_str(admin_address)
                .with_context(|| format!("Failed to parse administrator address: {}", admin_address))?
                .require_network(network)
                .with_context(|| {
                    format!(
                        "Administrator address {} is not valid for network {:?}",
                        admin_address, network
                    )
                })?;
            
            outputs.push(TxOut {
                value: Amount::from_sat(fees.administrator_fee),
                script_pubkey: address.script_pubkey(),
            });
        }
    }

    Ok(outputs)
}

/// Generate transaction outputs for an escape transaction (all bets)
pub fn generate_escape_outputs(all_bets: &[Bet], network: Network) -> Result<Vec<TxOut>> {
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
    Ok(outputs)
}

/// Build a withdrawal transaction
pub fn build_withdraw_transaction(params: WithdrawParams) -> Result<Transaction> {
    let outputs = match &params.withdraw_type {
        WithdrawType::Payout => {
            let winning_outcome = params
                .market
                .winning_outcome
                .ok_or_else(|| anyhow::anyhow!("Market must be settled for payout transactions"))?;
            let winning_bets = match winning_outcome {
                'A' => &params.market.bets_a,
                'B' => &params.market.bets_b,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid winning outcome: {}",
                        winning_outcome
                    ))
                }
            };
            generate_payout_outputs(
                winning_bets,
                params.market.total_amount,
                params.market.network,
                &params.market.fees,
            )?
        }
        WithdrawType::Escape => {
            let all_bets: Vec<Bet> = params
                .market
                .bets_a
                .iter()
                .chain(params.market.bets_b.iter())
                .cloned()
                .collect();
            generate_escape_outputs(&all_bets, params.market.network)?
        }
    };

    if outputs.is_empty() {
        return Err(anyhow::anyhow!("No valid outputs generated"));
    }

    // Create input spending the pool UTXO
    let input = TxIn {
        previous_output: params.pool_utxo,
        script_sig: ScriptBuf::new(),
        sequence: if matches!(params.withdraw_type, WithdrawType::Escape) {
            // Use locktime sequence for escape transactions
            let settlement_timestamp: u32 = params
                .market
                .settlement_timestamp
                .try_into()
                .with_context(|| "Settlement timestamp too large")?;
            let escape_time = settlement_timestamp + params.market.withdraw_timeout;
            Sequence(escape_time)
        } else {
            Sequence::ENABLE_RBF_NO_LOCKTIME
        },
        witness: Witness::new(),
    };

    // Create transaction
    let tx = Transaction {
        version: Version(get_tx_version(params.market.network)),
        lock_time: if matches!(params.withdraw_type, WithdrawType::Escape) {
            let settlement_timestamp: u32 = params
                .market
                .settlement_timestamp
                .try_into()
                .with_context(|| "Settlement timestamp too large")?;
            let escape_time = settlement_timestamp + params.market.withdraw_timeout;
            LockTime::from_time(escape_time).with_context(|| "Invalid escape locktime")?
        } else {
            LockTime::ZERO
        },
        input: vec![input],
        output: outputs,
    };

    Ok(tx)
}

/// Create witness data for spending the pool using the outcome path
pub fn create_outcome_witness(
    market: &PredictionMarket,
    winning_outcome: char,
    oracle_signature: &[u8],
    control_block: ControlBlock,
    script: ScriptBuf,
) -> Result<Witness> {
    let outcome_str = match winning_outcome {
        'A' => market.outcome_a.nostr_id(),
        'B' => market.outcome_b.nostr_id(),
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid winning outcome: {}",
                winning_outcome
            ))
        }
    };

    let outcome_hash = sha256::Hash::hash(outcome_str.as_bytes());

    let mut witness = Witness::new();
    witness.push(oracle_signature);
    witness.push(outcome_hash.as_byte_array());
    witness.push(script.as_bytes());
    witness.push(control_block.serialize());
    Ok(witness)
}

/// Create witness data for spending the pool using the escape path
pub fn create_escape_witness(control_block: ControlBlock, script: ScriptBuf) -> Result<Witness> {
    let mut witness = Witness::new();
    witness.push(script.as_bytes());
    witness.push(control_block.serialize());
    Ok(witness)
}

/// Sign and finalize a withdrawal transaction
pub fn sign_withdraw_transaction(
    mut tx: Transaction,
    params: &WithdrawParams,
    oracle_signature: Option<&[u8]>, // Required for payout, not needed for escape
) -> Result<Transaction> {
    // Create the appropriate script and control block based on withdraw type
    let (script, control_block) = match &params.withdraw_type {
        WithdrawType::Payout => {
            let winning_outcome = params
                .market
                .winning_outcome
                .ok_or_else(|| anyhow::anyhow!("Market must be settled for payout transactions"))?;

            // Generate the outcome script
            let ctv_hash = calculate_ctv_hash_from_transaction(&tx);

            let outcome_id = match winning_outcome {
                'A' => &params.market.outcome_a.nostr_id(),
                'B' => &params.market.outcome_b.nostr_id(),
                _ => return Err(anyhow::anyhow!("Invalid winning outcome")),
            };

            let script =
                build_script_for_outcome(ctv_hash, &params.market.oracle_pubkey, outcome_id)?;

            // For this example, we'll create a dummy control block
            // In a real implementation, you'd need to derive this from the market's taproot tree
            let control_block = create_dummy_control_block()?;

            (script, control_block)
        }
        WithdrawType::Escape => {
            let ctv_hash = calculate_ctv_hash_from_transaction(&tx);

            let script = build_script_for_escape(ctv_hash);
            let control_block = create_dummy_control_block()?;

            (script, control_block)
        }
    };

    // Create witness based on withdraw type
    let witness = match &params.withdraw_type {
        WithdrawType::Payout => {
            let winning_outcome = params
                .market
                .winning_outcome
                .ok_or_else(|| anyhow::anyhow!("Market must be settled for payout transactions"))?;
            let oracle_sig = oracle_signature.unwrap(); // We already checked this above
            create_outcome_witness(
                &params.market,
                winning_outcome,
                oracle_sig,
                control_block,
                script,
            )?
        }
        WithdrawType::Escape => create_escape_witness(control_block, script)?,
    };

    // Attach witness to the input
    tx.input[0].witness = witness;

    Ok(tx)
}

/// Helper function to create a dummy control block for testing
/// In a real implementation, this would be derived from the actual taproot tree
fn create_dummy_control_block() -> Result<ControlBlock> {
    // This is a placeholder - in reality you'd construct this from the market's taproot spending info
    let dummy_bytes = vec![0xc0; 33]; // 0xc0 is a valid control block first byte, followed by 32 bytes
    ControlBlock::decode(&dummy_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to create control block: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_generate_payout_outputs() {
        let market = create_test_market();
        let result = generate_payout_outputs(&market.bets_a, market.total_amount, Network::Regtest, &market.fees);

        assert!(
            result.is_ok(),
            "Should generate payout outputs successfully"
        );
        let outputs = result.unwrap();
        assert_eq!(
            outputs.len(),
            2,
            "Should generate 2 outputs for 2 winning bets"
        );

        // Check proportional distribution
        let total_winning = 150000u64; // 100k + 50k
        let pool_after_fees = market.fees.pool_after_fees(market.total_amount, 2); // 2 winning outputs
        let expected_amount_1 = (100000 * pool_after_fees) / total_winning;
        let expected_amount_2 = (50000 * pool_after_fees) / total_winning;

        assert_eq!(outputs[0].value.to_sat(), expected_amount_1);
        assert_eq!(outputs[1].value.to_sat(), expected_amount_2);
    }

    #[test]
    fn test_generate_escape_outputs() {
        let market = create_test_market();
        let all_bets: Vec<Bet> = market
            .bets_a
            .iter()
            .chain(market.bets_b.iter())
            .cloned()
            .collect();
        let result = generate_escape_outputs(&all_bets, Network::Regtest);

        assert!(
            result.is_ok(),
            "Should generate escape outputs successfully"
        );
        let outputs = result.unwrap();
        assert_eq!(
            outputs.len(),
            3,
            "Should generate 3 outputs for 3 total bets"
        );

        // Check amounts match original bets
        assert_eq!(outputs[0].value.to_sat(), 100000);
        assert_eq!(outputs[1].value.to_sat(), 50000);
        assert_eq!(outputs[2].value.to_sat(), 150000);
    }

    #[test]
    fn test_build_payout_transaction() {
        let market = create_test_market();
        let pool_utxo = OutPoint::new(
            "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                .parse()
                .unwrap(),
            0,
        );

        // Set the market as settled with A winning for the test
        let mut market = market;
        market.settled = true;
        market.winning_outcome = Some('A');

        let params = WithdrawParams {
            market,
            withdraw_type: WithdrawType::Payout,
            pool_utxo,
            fee_rate: None,
        };

        let result = build_withdraw_transaction(params);
        assert!(
            result.is_ok(),
            "Should build payout transaction successfully"
        );

        let tx = result.unwrap();
        assert_eq!(tx.input.len(), 1, "Should have 1 input");
        assert_eq!(tx.output.len(), 2, "Should have 2 outputs for winning side");
        assert_eq!(
            tx.lock_time,
            LockTime::ZERO,
            "Payout tx should have zero locktime"
        );
    }

    #[test]
    fn test_build_escape_transaction() {
        let market = create_test_market();
        let pool_utxo = OutPoint::new(
            "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                .parse()
                .unwrap(),
            0,
        );

        let params = WithdrawParams {
            market,
            withdraw_type: WithdrawType::Escape,
            pool_utxo,
            fee_rate: None,
        };

        let result = build_withdraw_transaction(params);
        assert!(
            result.is_ok(),
            "Should build escape transaction successfully"
        );

        let tx = result.unwrap();
        assert_eq!(tx.input.len(), 1, "Should have 1 input");
        assert_eq!(tx.output.len(), 3, "Should have 3 outputs for all bets");
        assert!(
            tx.lock_time != LockTime::ZERO,
            "Escape tx should have locktime"
        );
    }

    #[test]
    fn test_generate_payout_outputs_empty_bets() {
        let empty_bets = vec![];
        let fees = MarketFees::default();
        let result = generate_payout_outputs(&empty_bets, 300000, Network::Regtest, &fees);
        assert!(result.is_err(), "Should fail with empty bets");
    }
    
    #[test]
    fn test_generate_payout_outputs_with_admin_fee() {
        let bets = vec![
            Bet {
                payout_address: "bcrt1q0ywfmmk5d0es7chp5xqnw7x5l6nlanvnqcgnzn".to_string(),
                amount: 100000,
                txid: "abc123".to_string(),
                vout: 0,
            },
            Bet {
                payout_address: "bcrt1qalqsxa9tlzqq89mvdkqk37c7gvnyadlccudnsg".to_string(),
                amount: 50000,
                txid: "def456".to_string(),
                vout: 0,
            },
        ];
        
        let fees = MarketFees {
            fee_per_deposit_output: 500,
            fee_per_withdraw_output: 600,
            administrator_fee: 5000,
            administrator_address: Some("bcrt1qpjult34k9spjfym8hss2jrwjgf0xjf40ze0pp8".to_string()),
        };
        
        let result = generate_payout_outputs(&bets, 300000, Network::Regtest, &fees);
        assert!(result.is_ok(), "Should generate outputs with admin fee");
        
        let outputs = result.unwrap();
        // Should have 3 outputs: 2 for winners + 1 for admin
        assert_eq!(outputs.len(), 3, "Should have 3 outputs including admin fee");
        
        // Check admin fee output (should be last)
        let admin_output = &outputs[2];
        assert_eq!(admin_output.value.to_sat(), 5000, "Admin fee should be 5000 sats");
        
        // Verify the admin address
        let expected_admin_address = Address::from_str("bcrt1qpjult34k9spjfym8hss2jrwjgf0xjf40ze0pp8")
            .unwrap()
            .require_network(Network::Regtest)
            .unwrap();
        assert_eq!(admin_output.script_pubkey, expected_admin_address.script_pubkey());
        
        // Verify winner payouts are calculated correctly
        let total_winning = 150000u64;
        let pool_after_fees = fees.pool_after_fees(300000, 2); // 300000 - (2*600) - 5000 = 293800
        
        let expected_amount_1 = (100000 * pool_after_fees) / total_winning;
        let expected_amount_2 = (50000 * pool_after_fees) / total_winning;
        
        assert_eq!(outputs[0].value.to_sat(), expected_amount_1);
        assert_eq!(outputs[1].value.to_sat(), expected_amount_2);
    }
}
