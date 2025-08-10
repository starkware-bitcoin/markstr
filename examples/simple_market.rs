//! Simple prediction market example
//!
//! This example demonstrates creating a basic prediction market,
//! placing bets, and calculating payouts.

use markstr_core::{PredictionMarket, utils::*};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 Simple Prediction Market Example");
    println!("═══════════════════════════════════\n");

    // 1. Create a new prediction market
    println!("1. Creating a new prediction market...");
    
    let market = PredictionMarket::new(
        "Will it rain tomorrow in San Francisco?".to_string(),
        "Yes, it will rain".to_string(),
        "No, it will not rain".to_string(),
        "029b7b2f3c0e2c4d8f9a5b6c3d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8".to_string(),
        1735689600, // Settlement timestamp: January 1, 2025
    )?;

    println!("   Market ID: {}", market.market_id);
    println!("   Question: {}", market.question);
    println!("   Outcome A: {}", market.outcome_a);
    println!("   Outcome B: {}", market.outcome_b);
    println!("   Settlement: {}", format_timestamp(market.settlement_timestamp));
    println!();

    // 2. Get the market's Bitcoin address
    println!("2. Getting the market's Bitcoin address...");
    let market_address = market.get_market_address()?;
    println!("   Market Address: {}", market_address);
    println!("   (Send Bitcoin to this address to place bets)");
    println!();

    // 3. Simulate placing some bets
    println!("3. Simulating bet placement...");
    
    // Create a mutable copy for demonstration
    let mut market_copy = market.clone();
    
    // Alice bets 100,000 sats on "Yes"
    market_copy.place_bet(
        'A',
        100_000,
        "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(), // Alice's payout address
        "alice_tx_id_placeholder".to_string(),
        0,
    )?;
    
    // Bob bets 200,000 sats on "No"
    market_copy.place_bet(
        'B',
        200_000,
        "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3".to_string(), // Bob's payout address
        "bob_tx_id_placeholder".to_string(),
        0,
    )?;
    
    // Charlie bets 50,000 sats on "Yes"
    market_copy.place_bet(
        'A',
        50_000,
        "bc1qxw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4".to_string(), // Charlie's payout address
        "charlie_tx_id_placeholder".to_string(),
        0,
    )?;

    println!("   Alice bet 100,000 sats on 'Yes'");
    println!("   Bob bet 200,000 sats on 'No'");
    println!("   Charlie bet 50,000 sats on 'Yes'");
    println!();

    // 4. Calculate current odds and statistics
    println!("4. Current market statistics...");
    let total_a = market_copy.get_total_a();
    let total_b = market_copy.get_total_b();
    let total_pool = market_copy.total_amount;
    
    println!("   Total bets on 'Yes': {} sats ({} BTC)", total_a, satoshi_to_btc(total_a));
    println!("   Total bets on 'No': {} sats ({} BTC)", total_b, satoshi_to_btc(total_b));
    println!("   Total pool: {} sats ({} BTC)", total_pool, satoshi_to_btc(total_pool));
    println!();
    
    println!("   Odds for 'Yes': {:.2}x", market_copy.get_odds_a());
    println!("   Odds for 'No': {:.2}x", market_copy.get_odds_b());
    println!();

    // 5. Calculate potential payouts
    println!("5. Potential payouts if 'Yes' wins...");
    
    // Alice's payout (100,000 sats bet on winning side)
    let alice_payout = market_copy.calculate_payout(100_000, total_a);
    println!("   Alice would receive: {} sats ({} BTC)", alice_payout, satoshi_to_btc(alice_payout));
    
    // Charlie's payout (50,000 sats bet on winning side)  
    let charlie_payout = market_copy.calculate_payout(50_000, total_a);
    println!("   Charlie would receive: {} sats ({} BTC)", charlie_payout, satoshi_to_btc(charlie_payout));
    
    // Bob gets nothing (bet on losing side)
    println!("   Bob would receive: 0 sats (bet on losing side)");
    println!();

    // 6. Calculate potential payouts if 'No' wins
    println!("6. Potential payouts if 'No' wins...");
    
    // Bob's payout (200,000 sats bet on winning side)
    let bob_payout = market_copy.calculate_payout(200_000, total_b);
    println!("   Bob would receive: {} sats ({} BTC)", bob_payout, satoshi_to_btc(bob_payout));
    
    // Alice and Charlie get nothing
    println!("   Alice would receive: 0 sats (bet on losing side)");
    println!("   Charlie would receive: 0 sats (bet on losing side)");
    println!();

    // 7. Show market status
    println!("7. Market status...");
    println!("   Status: {}", market_copy.get_status());
    println!("   Is past settlement: {}", market_copy.is_past_settlement());
    println!();

    // 8. Generate outcome messages for oracle signing
    println!("8. Oracle outcome messages...");
    let message_a = market_copy.create_outcome_message(&market_copy.outcome_a);
    let message_b = market_copy.create_outcome_message(&market_copy.outcome_b);
    
    println!("   Message for 'Yes' outcome:");
    println!("   {}", message_a);
    println!();
    println!("   Message for 'No' outcome:");
    println!("   {}", message_b);
    println!();

    // 9. Utility demonstrations
    println!("9. Utility functions...");
    
    // Generate a new market ID
    let new_id = generate_market_id();
    println!("   Generated market ID: {}", new_id);
    
    // Hash a message
    let hash = sha256_hash("Hello, Markstr!");
    println!("   SHA256 hash: {}", hash);
    
    // Validate an address
    let is_valid = validate_address(&market_address, bitcoin::Network::Signet);
    println!("   Address validation: {}", is_valid);
    
    // Convert units
    println!("   1 BTC = {} satoshis", btc_to_satoshi(1.0));
    println!("   100,000 sats = {} BTC", satoshi_to_btc(100_000));
    println!();

    println!("✅ Example completed successfully!");
    println!("   Market address: {}", market_address);
    println!("   Send Bitcoin to this address to participate in the market.");
    println!("   After settlement time, the oracle will sign the outcome.");
    println!("   Winners can then claim their proportional share of the pool.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_simple_market() {
        // Test that the example runs without errors
        let result = main().await;
        assert!(result.is_ok());
    }
}
