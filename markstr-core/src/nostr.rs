//! # Nostr Client for Oracle Communication
//!
//! This module provides a simplified Nostr client for interacting with oracles
//! in the prediction market system.

use crate::{error::Result, MarketError};
use nostr::{Event, Keys, Kind, Tag, Timestamp, UnsignedEvent};
use std::collections::HashMap;

/// Simplified Nostr client for oracle communication
pub struct NostrClient {
    /// Nostr keys for signing events
    keys: Keys,
    /// Connected relays
    relays: Vec<String>,
    /// Cached events
    events: HashMap<String, Event>,
}

impl NostrClient {
    /// Create a new Nostr client
    pub fn new(secret_key: Option<&str>) -> Result<Self> {
        let keys = match secret_key {
            Some(sk) => Keys::parse(sk)?,
            None => Keys::generate(),
        };

        Ok(Self {
            keys,
            relays: vec![
                "wss://relay.damus.io".to_string(),
                "wss://nostr-pub.wellorder.net".to_string(),
                "wss://relay.nostr.info".to_string(),
            ],
            events: HashMap::new(),
        })
    }

    /// Get the public key
    pub fn public_key(&self) -> String {
        self.keys.public_key().to_hex()
    }

    /// Create an oracle event for market settlement
    pub fn create_oracle_event(
        &self,
        market_id: &str,
        outcome: &str,
        settlement_timestamp: u64,
    ) -> Result<Event> {
        let content = format!(
            "PredictionMarketId:{} Outcome:{} Timestamp:{}",
            market_id, outcome, settlement_timestamp
        );

        let tags = vec![
            Tag::generic("t", vec!["prediction-market".to_string()]),
            Tag::generic("market", vec![market_id.to_string()]),
            Tag::generic("outcome", vec![outcome.to_string()]),
        ];

        let unsigned_event = UnsignedEvent {
            pubkey: self.keys.public_key(),
            created_at: Timestamp::from(settlement_timestamp),
            kind: Kind::TextNote,
            tags,
            content,
        };

        let event = self.keys.sign_event(unsigned_event)?;
        Ok(event)
    }

    /// Publish an event to relays (mock implementation)
    pub async fn publish_event(&mut self, event: &Event) -> Result<()> {
        // In a real implementation, this would publish to actual Nostr relays
        // For now, we'll just store it locally
        self.events.insert(event.id.to_hex(), event.clone());
        println!("Published event {} to {} relays", event.id, self.relays.len());
        Ok(())
    }

    /// Subscribe to events (mock implementation)
    pub async fn subscribe_to_market(&mut self, market_id: &str) -> Result<Vec<Event>> {
        // In a real implementation, this would subscribe to relay filters
        // For now, return cached events that match the market
        let matching_events: Vec<Event> = self
            .events
            .values()
            .filter(|event| {
                event.tags.iter().any(|tag| {
                    tag.as_vec().len() >= 2 && tag.as_vec()[0] == "market" && tag.as_vec()[1] == market_id
                })
            })
            .cloned()
            .collect();

        Ok(matching_events)
    }

    /// Get an event by ID
    pub fn get_event(&self, event_id: &str) -> Option<&Event> {
        self.events.get(event_id)
    }

    /// Add a relay
    pub fn add_relay(&mut self, relay_url: String) {
        if !self.relays.contains(&relay_url) {
            self.relays.push(relay_url);
        }
    }

    /// Get connected relays
    pub fn get_relays(&self) -> &[String] {
        &self.relays
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nostr_client_creation() {
        let client = NostrClient::new(None).unwrap();
        assert!(!client.public_key().is_empty());
    }

    #[test]
    fn test_oracle_event_creation() {
        let client = NostrClient::new(None).unwrap();
        let event = client
            .create_oracle_event("TEST1234", "A", 1735689600)
            .unwrap();

        assert_eq!(event.kind, Kind::TextNote);
        assert!(event.content.contains("PredictionMarketId:TEST1234"));
        assert!(event.content.contains("Outcome:A"));
        assert!(event.verify());
    }
}
