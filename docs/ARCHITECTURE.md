# Markstr Architecture

## Overview

Markstr is designed as a modular system with clear separation of concerns:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Web Frontend  │    │   CLI Interface  │    │  WASM Module    │
│   (React App)   │    │   (Rust Binary)  │    │ (Browser/Node)  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                        │
         └───────────────────────┼────────────────────────┘
                                 │
                    ┌─────────────────────┐
                    │   markstr-core      │
                    │   (Rust Library)    │
                    └─────────────────────┘
                                 │
                    ┌─────────────────────┐
                    │     Bitcoin         │
                    │   (Taproot/CSFS)    │
                    └─────────────────────┘
```

## Core Components

### 1. markstr-core

The core library contains all business logic:

- **PredictionMarket**: Main market struct and operations
- **NostrClient**: Oracle communication via Nostr
- **CSFS Verification**: Cryptographic signature verification
- **Utility Functions**: Address validation, conversions, etc.

### 2. markstr-wasm

WebAssembly bindings for browser/Node.js:

- **WasmPredictionMarket**: WASM-compatible market struct
- **MarketAnalytics**: Real-time market statistics
- **Utility Functions**: Browser-compatible utilities
- **TypeScript Definitions**: Full type safety

### 3. markstr-cli

Command-line interface:

- **Market Creation**: Create markets from command line
- **Address Validation**: Validate Bitcoin addresses
- **Unit Conversion**: Convert between BTC and satoshis
- **Market Information**: Display market status and details

### 4. webapp

React web application:

- **Market Dashboard**: Overview of all markets
- **Role Management**: Switch between Oracle, Bettor, Viewer
- **Betting Interface**: Place bets with real-time odds
- **Analytics**: Market performance and statistics
- **Transaction History**: Complete audit trail

## Data Flow

### Market Creation

1. User specifies market parameters (question, outcomes, oracle, settlement time)
2. Core library generates unique market ID
3. Taproot address created with dual CSFS scripts (outcome A and B)
4. Market metadata stored/broadcast via Nostr
5. Market address returned for betting

### Betting Process

1. User sends Bitcoin to market Taproot address
2. Transaction confirmed on Bitcoin network
3. Bet registered in market state
4. Odds recalculated based on new bet volumes
5. Market statistics updated

### Settlement Process

1. Settlement time reached
2. Oracle creates Nostr event with outcome
3. Event signature verified against expected message format
4. Market marked as settled with winning outcome
5. Payout transactions can be created and broadcast

## Bitcoin Integration

### Taproot Scripts

Each market creates two CSFS scripts:

```
Script A: <outcome_a_message_hash> <oracle_pubkey> OP_CHECKSIGFROMSTACK
Script B: <outcome_b_message_hash> <oracle_pubkey> OP_CHECKSIGFROMSTACK
```

### Address Generation

```rust
// Create Taproot address with both scripts
let spend_info = TaprootBuilder::new()
    .add_leaf(1, script_a)?
    .add_leaf(1, script_b)?
    .finalize(&secp, nums_point)?;

let address = Address::p2tr_tweaked(spend_info.output_key(), network);
```

### Payout Transactions

Winners create transactions spending from the market address:

```
Input: Market UTXO
Witness: [oracle_signature, winning_script, control_block]
Output: Payout to winner's address
```

## Nostr Integration

### Oracle Events

Oracle creates events with specific format:

```json
{
  "kind": 1,
  "content": "PredictionMarketId:ABC123 Outcome:A Timestamp:1735689600",
  "tags": [
    ["t", "prediction-market"],
    ["market", "ABC123"],
    ["outcome", "A"]
  ]
}
```

### Event Verification

Market settlement verifies:
1. Event signature is valid
2. Oracle pubkey matches market oracle
3. Timestamp >= settlement time
4. Content matches expected format

## Security Model

### Cryptographic Guarantees

- **Oracle Signatures**: Verified via CSFS on Bitcoin
- **Market Addresses**: Deterministic Taproot generation
- **Outcome Messages**: SHA256 hashed for integrity
- **Private Keys**: Never exposed in web interfaces

### Trust Assumptions

- **Oracle Honesty**: Oracle will sign correct outcome
- **Oracle Availability**: Oracle will sign at settlement time
- **Bitcoin Security**: Bitcoin network security assumptions
- **Nostr Availability**: Nostr relays remain accessible

### Attack Vectors

- **Oracle Compromise**: Oracle private key theft
- **Early Settlement**: Oracle signs before settlement time
- **Wrong Outcome**: Oracle signs incorrect outcome
- **Denial of Service**: Oracle refuses to sign

### Mitigations

- **Oracle Reputation**: Track oracle performance
- **Multi-Oracle Support**: Require multiple oracle signatures
- **Timelock Fallback**: Refund mechanism if oracle fails
- **Dispute Resolution**: Community governance for disputes

## Performance Characteristics

### Core Library

- **Market Creation**: O(1) - constant time
- **Bet Placement**: O(1) - constant time
- **Odds Calculation**: O(1) - constant time
- **Settlement**: O(1) - constant time

### WASM Module

- **Bundle Size**: ~1.2MB (can be optimized)
- **Initialization**: ~10ms on modern browsers
- **Calculations**: 2-10x faster than JavaScript
- **Memory Usage**: Minimal heap allocation

### Web Application

- **Initial Load**: ~3-5 seconds (including WASM)
- **Market Updates**: Real-time via WebSocket
- **Betting Flow**: <2 seconds end-to-end
- **Mobile Performance**: Optimized for mobile devices

## Scalability Considerations

### Current Limitations

- **Bitcoin Throughput**: Limited by Bitcoin block space
- **Nostr Scaling**: Dependent on relay network
- **State Management**: No persistent global state
- **Oracle Bottleneck**: Single oracle per market

### Future Improvements

- **Layer 2 Integration**: Lightning Network support
- **Batch Settlements**: Multiple markets per transaction
- **State Channels**: Off-chain betting with on-chain settlement
- **Relay Optimization**: Efficient Nostr relay selection

## Development Patterns

### Error Handling

```rust
// Custom error types for clear error reporting
pub enum MarketError {
    Bitcoin(bitcoin::Error),
    Nostr(nostr::Error),
    InvalidMarket(String),
    InvalidBet(String),
    Oracle(String),
    Settlement(String),
}
```

### Async Operations

```rust
// Async/await for network operations
pub async fn publish_event(&mut self, event: &Event) -> Result<()> {
    // Publish to multiple relays concurrently
    let futures = self.relays.iter().map(|relay| {
        publish_to_relay(relay, event)
    });
    
    futures::try_join_all(futures).await?;
    Ok(())
}
```

### WASM Interop

```rust
// Safe conversion between Rust and JavaScript types
#[wasm_bindgen]
pub fn calculate_payout(&self, bet_amount: u64, winning_total: u64, total_pool: u64) -> u64 {
    if winning_total == 0 || total_pool == 0 {
        return 0;
    }
    ((bet_amount as f64 / winning_total as f64) * total_pool as f64) as u64
}
```

## Testing Strategy

### Unit Tests

- **Core Logic**: Test all market operations
- **Cryptographic Functions**: Verify signature operations
- **Utility Functions**: Test conversions and validations
- **Error Handling**: Verify proper error propagation

### Integration Tests

- **Bitcoin Integration**: Test with regtest network
- **Nostr Integration**: Test with local relay
- **WASM Binding**: Test JavaScript interop
- **CLI Interface**: Test command-line operations

### End-to-End Tests

- **Market Lifecycle**: Full market creation to settlement
- **Web Interface**: Automated browser testing
- **Multi-User Scenarios**: Concurrent betting simulation
- **Error Recovery**: Network failure handling

## Deployment Considerations

### Environment Setup

- **Bitcoin Node**: Full node or pruned node
- **Nostr Relays**: Public relays or private infrastructure
- **Web Hosting**: Static site hosting for webapp
- **CDN**: Global distribution for WASM module

### Monitoring

- **Market Health**: Active markets and betting volume
- **Oracle Performance**: Response times and accuracy
- **Network Status**: Bitcoin and Nostr connectivity
- **Error Rates**: Application and network errors

### Maintenance

- **Dependency Updates**: Keep Bitcoin and Nostr libraries current
- **Security Patches**: Monitor for vulnerabilities
- **Performance Optimization**: Profile and optimize hot paths
- **Documentation**: Keep architecture docs updated