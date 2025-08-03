//! Error types for markstr-core

use thiserror::Error;

/// Result type alias for markstr operations
pub type Result<T> = std::result::Result<T, MarketError>;

/// Error types for market operations
#[derive(Error, Debug)]
pub enum MarketError {
    /// Bitcoin-related errors
    #[error("Bitcoin error: {0}")]
    BitcoinHex(#[from] bitcoin::error::PrefixedHexError),

    /// Bitcoin-related errors
    #[error("Bitcoin error: {0}")]
    Bitcoin(#[from] bitcoin::error::UnprefixedHexError),

    /// Taproot errors
    #[error("Taproot error: {0}")]
    TaprootBuilderError(#[from] bitcoin::taproot::TaprootBuilderError),

    /// Secp256k1 errors
    #[error("Secp256k1 error: {0}")]
    Secp256k1(#[from] bitcoin::secp256k1::Error),

    /// Hex decoding errors
    #[error("Hex decoding error: {0}")]
    Hex(#[from] hex::FromHexError),

    /// Serde JSON errors
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Nostr errors
    // #[error("Nostr error: {0}")]
    // Nostr(#[from] nostr::Error),

    /// Market validation errors
    #[error("Invalid market: {0}")]
    InvalidMarket(String),

    /// Betting errors
    #[error("Invalid bet: {0}")]
    InvalidBet(String),

    /// Oracle errors
    #[error("Oracle error: {0}")]
    Oracle(String),

    /// Settlement errors
    #[error("Settlement error: {0}")]
    Settlement(String),

    /// Payout errors
    #[error("Payout error: {0}")]
    Payout(String),

    /// Address validation errors
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Signature verification errors
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Network errors
    #[error("Network error: {0}")]
    Network(String),

    /// Generic error for other cases
    #[error("Market error: {0}")]
    Other(String),
}

impl From<&str> for MarketError {
    fn from(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }
}

impl From<String> for MarketError {
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

