//! # Markstr Core
//!
//! Core Rust library for Nostr-based Bitcoin prediction markets using CSFS and Taproot.
//!
//! This library provides the fundamental building blocks for creating decentralized
//! prediction markets where:
//! - Markets are created and settled using Nostr events
//! - Funds are held in Bitcoin Taproot addresses
//! - Payouts are verified using CSFS (```CheckSigFromStack```) signatures
//!
//! ## Features
//!
//! - **Market Creation**: Create binary prediction markets with oracle-based settlement
//! - **Betting System**: Place bets on market outcomes with Bitcoin transactions
//! - **Oracle Integration**: Nostr-based oracle system for outcome verification
//! - **CSFS Verification**: Cryptographic verification of oracle signatures
//! - **Payout Distribution**: Proportional payout calculation and distribution
//!
//! ## Examples
//!
//! ```rust
//! use markstr_core::PredictionMarket;
//!
//! // Create a new prediction market
//! let market = PredictionMarket::new(
//!     "Who will win the 2024 election?".to_string(),
//!     "Candidate A".to_string(),
//!     "Candidate B".to_string(),
//!     "ee96d4b9c5e16f3b11e33bb27fe39ae7a57daa6b24210de5b39237993742cc0a".to_string(),
//!     1735689600, // Settlement timestamp
//! )?;
//!
//! // Get the market's Bitcoin address for betting
//! let market_address = market.get_market_address()?;
//! println!("Send bets to: {}", market_address);
//! Ok::<(), markstr_core::MarketError>(())
//! ```

pub mod error;
pub mod market;
pub mod protocol;
pub mod utils;

pub use error::{MarketError, Result};
pub use market::{Bet, PredictionMarket};
pub use utils::*;

/// Default fee for market transactions (1000 satoshis)
pub const DEFAULT_MARKET_FEE: u64 = 1000;

/// ```OP_CHECKSIGFROMSTACK``` opcode (0xcc)
pub const OP_CHECKSIGFROMSTACK: u8 = 0xcc;
