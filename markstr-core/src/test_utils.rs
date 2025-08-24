//! Common test utilities for markstr-core tests.
//!
//! This module provides shared functionality for testing across all modules,
//! including market creation, address generation, and other common test setup.

use crate::market::{Bet, MarketFees, PredictionMarket, PredictionOutcome};
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::{Address, CompressedPublicKey, Network, PrivateKey};

/// Generate a valid regtest address for testing purposes.
/// Uses deterministic key generation based on the index for reproducible tests.
pub fn create_valid_regtest_address(index: u8) -> String {
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

/// Generate a valid address for a specific network.
/// Uses deterministic key generation based on the index for reproducible tests.
pub fn create_valid_address_for_network(index: u8, network: Network) -> String {
    let secp = Secp256k1::new();
    let mut secret_bytes = [0u8; 32];
    secret_bytes[0] = index;
    secret_bytes[31] = index;
    let secret_key = SecretKey::from_slice(&secret_bytes).unwrap();
    let private_key = PrivateKey::new(secret_key, network);
    let public_key = CompressedPublicKey::from_private_key(&secp, &private_key).unwrap();
    let address = Address::p2wpkh(&public_key, network);
    address.to_string()
}

/// Create a standard test prediction market with predefined bets.
/// Uses regtest network and creates a market with bets on both sides.
pub fn create_test_market() -> PredictionMarket {
    create_test_market_with_network(Network::Regtest)
}

/// Create a test prediction market for a specific network.
pub fn create_test_market_with_network(network: Network) -> PredictionMarket {
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
        network,
        market_utxo: None,
        total_amount: 300000, // 3 BTC worth in sats
        bets_a: vec![
            Bet {
                payout_address: if network == Network::Regtest {
                    create_valid_regtest_address(1)
                } else {
                    create_valid_address_for_network(1, network)
                },
                amount: 100000,
                txid: "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                    .to_string(),
                vout: 0,
            },
            Bet {
                payout_address: if network == Network::Regtest {
                    create_valid_regtest_address(2)
                } else {
                    create_valid_address_for_network(2, network)
                },
                amount: 50000,
                txid: "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabce"
                    .to_string(),
                vout: 1,
            },
        ],
        bets_b: vec![Bet {
            payout_address: if network == Network::Regtest {
                create_valid_regtest_address(3)
            } else {
                create_valid_address_for_network(3, network)
            },
            amount: 150000,
            txid: "fedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafed".to_string(),
            vout: 0,
        }],
        settled: false,
        winning_outcome: None,
        withdraw_timeout: 86400, // 1 day
        fees: MarketFees::default(),
    }
}

/// Create a minimal test market with no bets.
pub fn create_empty_test_market() -> PredictionMarket {
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
        market_id: "empty_market_id".to_string(),
        question: "Empty market for testing?".to_string(),
        outcome_a,
        outcome_b,
        oracle_pubkey: "ee96d4b9c5e16f3b11e33bb27fe39ae7a57daa6b24210de5b39237993742cc0a"
            .to_string(),
        settlement_timestamp: 1735689600,
        network: Network::Regtest,
        market_utxo: None,
        total_amount: 0,
        bets_a: vec![],
        bets_b: vec![],
        settled: false,
        winning_outcome: None,
        withdraw_timeout: 86400,
        fees: MarketFees::default(),
    }
}

/// Create a test market with custom bet amounts.
pub fn create_test_market_with_amounts(
    bets_a_amounts: Vec<u64>,
    bets_b_amounts: Vec<u64>,
) -> PredictionMarket {
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

    let mut bets_a = Vec::new();
    for (i, amount) in bets_a_amounts.iter().enumerate() {
        bets_a.push(Bet {
            payout_address: create_valid_regtest_address((i + 1) as u8),
            amount: *amount,
            txid: format!(
                "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefab{:02x}",
                i
            ),
            vout: i as u32,
        });
    }

    let mut bets_b = Vec::new();
    for (i, amount) in bets_b_amounts.iter().enumerate() {
        bets_b.push(Bet {
            payout_address: create_valid_regtest_address((i + 10) as u8),
            amount: *amount,
            txid: format!(
                "fedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafedcbafe{:02x}",
                i
            ),
            vout: i as u32,
        });
    }

    let total_amount = bets_a_amounts.iter().sum::<u64>() + bets_b_amounts.iter().sum::<u64>();

    PredictionMarket {
        market_id: "custom_market_id".to_string(),
        question: "Custom amounts market?".to_string(),
        outcome_a,
        outcome_b,
        oracle_pubkey: "ee96d4b9c5e16f3b11e33bb27fe39ae7a57daa6b24210de5b39237993742cc0a"
            .to_string(),
        settlement_timestamp: 1735689600,
        network: Network::Regtest,
        market_utxo: None,
        total_amount,
        bets_a,
        bets_b,
        settled: false,
        winning_outcome: None,
        withdraw_timeout: 86400,
        fees: MarketFees::default(),
    }
}

/// Common test constants
pub mod constants {
    /// Standard oracle public key used in tests
    pub const TEST_ORACLE_PUBKEY: &str =
        "ee96d4b9c5e16f3b11e33bb27fe39ae7a57daa6b24210de5b39237993742cc0a";

    /// Standard settlement timestamp (Jan 1, 2025)
    pub const TEST_SETTLEMENT_TIMESTAMP: u64 = 1735689600;

    /// Standard withdraw timeout (1 day)
    pub const TEST_WITHDRAW_TIMEOUT: u32 = 86400;

    /// Standard test transaction ID
    pub const TEST_TXID: &str = "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd";
}
