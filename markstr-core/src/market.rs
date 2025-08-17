//! # Prediction Market Implementation
//!
//! This module implements the core prediction market functionality using Bitcoin
//! Taproot and CSFS (```CheckSigFromStack```) for oracle-based settlement.

use crate::{error::Result, MarketError, DEFAULT_MARKET_FEE, OP_CHECKSIGFROMSTACK};
use bitcoin::{
    hashes::{sha256, Hash},
    secp256k1::{Keypair, Message, Secp256k1, XOnlyPublicKey},
    taproot::TaprootBuilder,
    Address, Network, OutPoint, ScriptBuf,
};
use serde::{Deserialize, Serialize};

/// Represents a prediction outcome that will be used to predefine the market.
/// This outcome should be verifiably immutable.
/// We can standardize outcome format to a Nostr event.
/// The outcome description can be the Nostr event content.
/// The outcome timestamp can be the Nostr event created_at.
/// The oracle pubkey can be the Nostr event pubkey.
/// We can set a static kind XXXX for the event (42 for now).
/// And a static tag as the character of the outcome.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PredictionOutcome {
    pub outcome: String,
    pub oracle: String,
    pub timestamp: u64,
    pub character: char,
}
impl PredictionOutcome {
    pub fn new(outcome: String, oracle: String, timestamp: u64, character: char) -> Result<Self> {
        if outcome.is_empty() {
            return Err(MarketError::InvalidOutcome(
                "Outcome cannot be empty".to_string(),
            ));
        }
        if outcome.len() > 255 {
            return Err(MarketError::InvalidOutcome(
                "Outcome cannot be longer than 255 characters".to_string(),
            ));
        }

        Ok(Self {
            outcome,
            oracle,
            timestamp,
            character,
        })
    }
    pub fn nostr_id(&self) -> String {
        crate::sha256_hash_for_nostr_id(
            &self.outcome,
            &self.oracle,
            self.timestamp,
            42,
            &[&["outcome", &self.character.to_string()]],
        )
    }
    pub fn verify_signature(&self, signature: &str) -> Result<bool> {
        crate::verify_signature(&self.nostr_id(), signature, &self.oracle)
    }
}

/// Represents a binary prediction market using Nostr oracles and CSFS verification.
///
/// The market creates a Taproot address with two script paths:
/// - Path A: Verifies oracle signature for outcome A
/// - Path B: Verifies oracle signature for outcome B
///
/// Participants bet by sending funds to the market address. Winners claim
/// proportional payouts by providing the oracle's signed outcome.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PredictionMarket {
    /// Unique market identifier (8-character hex)
    pub market_id: String,

    /// Market question/description
    pub question: String,

    /// Binary outcome A (e.g., "Team A wins", "Yes")
    pub outcome_a: PredictionOutcome,

    /// Binary outcome B (e.g., "Team B wins", "No")
    pub outcome_b: PredictionOutcome,

    /// Oracle's Nostr public key (hex-encoded)
    pub oracle_pubkey: String,

    /// Deadline timestamp for oracle to sign outcome (Unix timestamp)
    pub settlement_timestamp: u64,

    /// Bitcoin network (Signet for testing)
    pub network: Network,

    /// Market funding UTXO (if funded)
    pub market_utxo: Option<OutPoint>,

    /// Total amount in the market (in satoshis)
    pub total_amount: u64,

    /// Bets placed on outcome A
    pub bets_a: Vec<Bet>,

    /// Bets placed on outcome B
    pub bets_b: Vec<Bet>,

    /// Whether the market has been settled
    pub settled: bool,

    /// Winning outcome (if settled)
    pub winning_outcome: Option<char>, // 'A' or 'B'
}

/// Represents a bet placed by a participant
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Bet {
    /// Bettor's payout address
    pub payout_address: String,

    /// Amount bet in satoshis
    pub amount: u64,

    /// Transaction ID of the bet
    pub txid: String,

    /// Output index in the transaction
    pub vout: u32,
}

impl PredictionMarket {
    /// Creates a new prediction market with the specified parameters.
    ///
    /// # Arguments
    /// * `question` - The market question (e.g., "Who will win the 2024 election?")
    /// * `outcome_a` - First possible outcome (e.g., "Candidate A wins")
    /// * `outcome_b` - Second possible outcome (e.g., "Candidate B wins")
    /// * `oracle_pubkey` - Oracle's Nostr public key (hex-encoded)
    /// * `settlement_timestamp` - When oracle should sign outcome (Unix timestamp)
    ///
    /// # Returns
    /// A new `PredictionMarket` instance ready for betting
    pub fn new(
        question: String,
        outcome_a: String,
        outcome_b: String,
        oracle_pubkey: String,
        settlement_timestamp: u64,
    ) -> Result<Self> {
        // Generate the outcomes
        let outcome_a =
            PredictionOutcome::new(outcome_a, oracle_pubkey.clone(), settlement_timestamp, 'A')?;
        let outcome_b =
            PredictionOutcome::new(outcome_b, oracle_pubkey.clone(), settlement_timestamp, 'B')?;

        // Market Id is a Nostr Note ID(sha256) of the question, oracle pubkey, and settlement timestamp
        // with the tag "outcome" and the outcome nostr_ids
        let market_id = crate::sha256_hash_for_nostr_id(
            &question,
            &oracle_pubkey,
            settlement_timestamp,
            42,
            &[&["outcomes", &outcome_a.nostr_id(), &outcome_b.nostr_id()]],
        );

        // Validate oracle pubkey format
        if hex::decode(&oracle_pubkey).is_err() || hex::decode(&oracle_pubkey)?.len() != 32 {
            return Err(MarketError::InvalidMarket(
                "Oracle pubkey must be 32-byte hex string".to_string(),
            ));
        }

        Ok(Self {
            market_id,
            question,
            outcome_a,
            outcome_b,
            oracle_pubkey,
            settlement_timestamp,
            network: Network::Signet,
            market_utxo: None,
            total_amount: 0,
            bets_a: Vec::new(),
            bets_b: Vec::new(),
            settled: false,
            winning_outcome: None,
        })
    }

    /// Generate NUMS (Nothing Up My Sleeve) point for Taproot internal key.
    pub fn nums_point() -> Result<XOnlyPublicKey> {
        let nums_bytes = [
            0x50, 0x92, 0x9b, 0x74, 0xc1, 0xa0, 0x49, 0x54, 0xb7, 0x8b, 0x4b, 0x60, 0x35, 0xe9,
            0x7a, 0x5e, 0x07, 0x8a, 0x5a, 0x0f, 0x28, 0xec, 0x96, 0xd5, 0x47, 0xbf, 0xee, 0x9a,
            0xce, 0x80, 0x3a, 0xc0,
        ];

        XOnlyPublicKey::from_slice(&nums_bytes)
            .map_err(|e| MarketError::InvalidAddress(format!("Failed to create NUMS point: {e}")))
    }

    /// Create CSFS script for a specific outcome.
    ///
    /// The script verifies that the provided signature (from witness) matches
    /// the expected oracle signature for the given outcome.
    ///
    /// # Script Structure
    /// ```text
    /// <outcome_message_hash> <oracle_pubkey> OP_CHECKSIGFROMSTACK
    /// ```
    pub fn create_outcome_script(&self, outcome: &str) -> Result<ScriptBuf> {
        // Create expected outcome message and hash it
        // A nostr event derives an `id` which is the sha256 hash of the content, pubkey, created_at,
        // kind, and tags. Each outcome can be the derived id of a nostr event, so it can be recreated
        // and verified in a client-side application.
        let outcome_hash = sha256::Hash::hash(outcome.as_bytes());

        // Parse oracle pubkey
        let oracle_pubkey = hex::decode(&self.oracle_pubkey)?;

        // Real CSFS implementation for production
        // Script: <outcome_message_hash> <oracle_pubkey> OP_CHECKSIGFROMSTACK
        let mut script_bytes = Vec::new();

        // Push outcome message hash (32 bytes)
        script_bytes.push(outcome_hash.as_byte_array().len().try_into().map_err(|_| {
            MarketError::InvalidAddress("Outcome hash length exceeds 32 bytes".to_string())
        })?);
        script_bytes.extend_from_slice(outcome_hash.as_byte_array());

        // Push oracle pubkey (32 bytes)
        script_bytes.push(oracle_pubkey.len().try_into().map_err(|_| {
            MarketError::InvalidAddress("Oracle pubkey length exceeds 32 bytes".to_string())
        })?);
        script_bytes.extend_from_slice(&oracle_pubkey);

        // Add OP_CHECKSIGFROMSTACK (0xcc) for real verification
        script_bytes.push(OP_CHECKSIGFROMSTACK);

        Ok(ScriptBuf::from_bytes(script_bytes))
    }

    /// Generate the market's Taproot address with dual outcome scripts.
    ///
    /// Creates a Taproot address with two script paths:
    /// - Path 0: CSFS verification for outcome A
    /// - Path 1: CSFS verification for outcome B
    ///
    /// # Returns
    /// The market's bech32m Taproot address where bets are sent
    pub fn get_market_address(&self) -> Result<String> {
        let script_a = self.create_outcome_script(&self.outcome_a.nostr_id())?;
        let script_b = self.create_outcome_script(&self.outcome_b.nostr_id())?;
        let nums_point = Self::nums_point()?;
        let secp = Secp256k1::new();

        let spend_info = TaprootBuilder::new()
            .add_leaf(1, script_a)?
            .add_leaf(1, script_b)?
            .finalize(&secp, nums_point)
            .map_err(|e| {
                MarketError::InvalidAddress(format!("Failed to finalize taproot: {e:?}"))
            })?;

        let address = Address::p2tr_tweaked(spend_info.output_key(), self.network);
        Ok(address.to_string())
    }

    /// Place a bet on a specific outcome.
    ///
    /// # Arguments
    /// * `outcome` - Which outcome to bet on ('A' or 'B')
    /// * `amount` - Amount to bet in satoshis
    /// * `payout_address` - Where to send winnings if this bet wins
    /// * `txid` - Transaction ID of the funding transaction
    /// * `vout` - Output index in the funding transaction
    pub fn place_bet(
        &mut self,
        outcome: char,
        amount: u64,
        payout_address: String,
        txid: String,
        vout: u32,
    ) -> Result<()> {
        if self.settled {
            return Err(MarketError::InvalidBet(
                "Market has already been settled".to_string(),
            ));
        }

        let bet = Bet {
            payout_address,
            amount,
            txid,
            vout,
        };

        match outcome.to_ascii_uppercase() {
            'A' => {
                self.bets_a.push(bet);
                self.total_amount += amount;
            }
            'B' => {
                self.bets_b.push(bet);
                self.total_amount += amount;
            }
            _ => {
                return Err(MarketError::InvalidBet(
                    "Outcome must be 'A' or 'B'".to_string(),
                ))
            }
        }

        Ok(())
    }

    /// Calculate payout for a winning bet.
    ///
    /// Winners split the total pool proportionally based on their bet size
    /// relative to the total amount bet on the winning side.
    pub fn calculate_payout(&self, bet_amount: u64, winning_side_total: u64) -> u64 {
        if winning_side_total == 0 {
            return 0;
        }

        // Winner's share = (their_bet / total_winning_bets) * total_pool
        // Subtract fees from total pool
        let pool_after_fees = self.total_amount.saturating_sub(DEFAULT_MARKET_FEE);
        (bet_amount * pool_after_fees) / winning_side_total
    }

    /// Settle the market with oracle signature.
    ///
    /// # Arguments
    /// * `oracle_event` - The Nostr event signed by the oracle
    /// * `outcome` - Which outcome won ('A' or 'B')
    pub fn settle_market(
        &mut self,
        outcome: &PredictionOutcome,
        outcome_signature: &str,
    ) -> Result<()> {
        if self.settled {
            return Err(MarketError::Settlement(
                "Market already settled".to_string(),
            ));
        }

        // Verify oracle signature
        if !outcome.verify_signature(outcome_signature)? {
            return Err(MarketError::InvalidSignature(
                "Invalid oracle signature".to_string(),
            ));
        }

        // Verify oracle pubkey matches
        if outcome.oracle != self.oracle_pubkey {
            return Err(MarketError::Oracle("Oracle pubkey mismatch".to_string()));
        }

        // Verify timestamp is at or after settlement time
        if outcome.timestamp < self.settlement_timestamp {
            return Err(MarketError::Oracle(
                "Oracle signed before settlement time".to_string(),
            ));
        }

        // Verify outcome message format
        let expected_outcome = match outcome.character.to_ascii_uppercase() {
            'A' => &self.outcome_a,
            'B' => &self.outcome_b,
            _ => return Err(MarketError::InvalidBet("Invalid outcome".to_string())),
        };

        let expected_message = expected_outcome.nostr_id();
        if outcome.nostr_id() != expected_message {
            return Err(MarketError::Oracle(
                "Oracle message doesn't match expected format".to_string(),
            ));
        }

        // Mark market as settled
        self.settled = true;
        self.winning_outcome = Some(outcome.character.to_ascii_uppercase());

        Ok(())
    }

    /// Get total amount bet on outcome A
    pub fn get_total_a(&self) -> u64 {
        self.bets_a.iter().map(|b| b.amount).sum()
    }

    /// Get total amount bet on outcome B
    pub fn get_total_b(&self) -> u64 {
        self.bets_b.iter().map(|b| b.amount).sum()
    }

    /// Get current odds for outcome A (as a ratio)
    pub fn get_odds_a(&self) -> f64 {
        let total_a = self.get_total_a() as f64;
        let total_b = self.get_total_b() as f64;

        if total_a == 0.0 {
            return 1.0;
        }

        (total_a + total_b) / total_a
    }

    /// Get current odds for outcome B (as a ratio)
    pub fn get_odds_b(&self) -> f64 {
        let total_a = self.get_total_a() as f64;
        let total_b = self.get_total_b() as f64;

        if total_b == 0.0 {
            return 1.0;
        }

        (total_a + total_b) / total_b
    }

    /// Check if market is past settlement time
    pub fn is_past_settlement(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.settlement_timestamp
    }

    /// Get market status summary
    pub fn get_status(&self) -> String {
        if self.settled {
            self.winning_outcome.map_or_else(
                || "Settled - No outcome set".to_string(),
                |outcome| format!("Settled - Outcome {outcome} won"),
            )
        } else if self.is_past_settlement() {
            "Awaiting oracle settlement".to_string()
        } else {
            "Active - Accepting bets".to_string()
        }
    }

    /// Verify CSFS signature against outcome message.
    ///
    /// This function verifies that the oracle signature is valid for the given outcome
    /// by checking the signature against the expected outcome message hash.
    ///
    /// # Arguments
    /// * `signature` - The oracle's signature bytes
    /// * `outcome` - The outcome being verified ('A' or 'B')
    ///
    /// # Returns
    /// `true` if the signature is valid for the outcome, `false` otherwise
    pub fn verify_csfs_signature(&self, signature: &[u8], outcome: &str) -> Result<bool> {
        // Create expected outcome message and hash it
        let outcome_hash = sha256::Hash::hash(outcome.as_bytes());

        // Parse oracle pubkey
        let oracle_pubkey_bytes = hex::decode(&self.oracle_pubkey)?;
        let oracle_pubkey = XOnlyPublicKey::from_slice(&oracle_pubkey_bytes)
            .map_err(|e| MarketError::InvalidSignature(format!("Invalid oracle pubkey: {e}")))?;

        // Create message from hash
        let message = Message::from_digest_slice(outcome_hash.as_byte_array()).map_err(|e| {
            MarketError::InvalidSignature(format!("Failed to create message from hash: {e}"))
        })?;

        // Parse signature
        if signature.len() != 64 {
            return Err(MarketError::InvalidSignature(format!(
                "Invalid signature length: expected 64 bytes, got {}",
                signature.len()
            )));
        }

        let secp = Secp256k1::new();
        let schnorr_sig = bitcoin::secp256k1::schnorr::Signature::from_slice(signature)
            .map_err(|e| MarketError::InvalidSignature(format!("Invalid signature format: {e}")))?;

        // Verify signature
        match secp.verify_schnorr(&schnorr_sig, &message, &oracle_pubkey) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Create CSFS signature for outcome message (for testing/oracle use).
    ///
    /// This function creates a valid CSFS signature that can be used to spend
    /// from the market address for the given outcome.
    ///
    /// # Arguments
    /// * `oracle_secret_key` - The oracle's secret key
    /// * `outcome` - The outcome being signed ('A' or 'B')
    ///
    /// # Returns
    /// 64-byte signature that can be used in the witness stack
    pub fn create_csfs_signature(
        &self,
        oracle_secret_key: &[u8],
        outcome: &str,
    ) -> Result<Vec<u8>> {
        if oracle_secret_key.len() != 32 {
            return Err(MarketError::InvalidSignature(
                "Oracle secret key must be 32 bytes".to_string(),
            ));
        }

        // Create expected outcome message and hash it
        let outcome_hash = sha256::Hash::hash(outcome.as_bytes());

        // Create message from hash
        let message = Message::from_digest_slice(outcome_hash.as_byte_array()).map_err(|e| {
            MarketError::InvalidSignature(format!("Failed to create message from hash: {e}"))
        })?;

        // Create keypair from secret key
        let secp = Secp256k1::new();
        let secret_key = bitcoin::secp256k1::SecretKey::from_slice(oracle_secret_key)
            .map_err(|e| MarketError::InvalidSignature(format!("Invalid secret key: {e}")))?;
        let keypair = Keypair::from_secret_key(&secp, &secret_key);

        // Create signature
        let signature = secp.sign_schnorr(&message, &keypair);

        Ok(signature.serialize().to_vec())
    }
}
