# Markstr Core

An interactive protocol for prediction markets on Bitcoin:
- External Oracles via OP_CSFS
- Coinpool via OP_CTV (allows multiple participants)
- Communication via Nostr

In a nutshell this is a coinpool with spending paths restricted by either oracle signatures (happy path) or a timelock (escape hatch).
It requires a single round of communication where participants exchange their signatures.

## Protocol

### Phase 1: New market is created: accepting bets

Participants signal their intentions to make bets by broadcasting:
- The outcome they want to bet on
- The bet amount
- The payout address

### Phase 2: The market is sealed: pooling funds

- No more bets are accepted at this point (bet timestamp must be less than the deadline).  
- Each participant should have the same view on the market state at this point, otherwise the protocol would fail.
- Participants independently compute the locking script hash, pre-sign the pooling (coinpool) transaction, and broadcast their inputs
- Pooling transaction has multiple inputs (one per participant) and a single output protected by OP_CTV
- Once all the inputs are collected, the pooling transaction is finalized and submitted to the network

Here are the Taproot script branches:
- In case of outcome A, winning parties can withdraw to their payout addresses
- Same for outcome B (outcome is verified by checking the oracle signature via OP_CSFS)
- Timelocked withdrawal to all, can be triggered by anyone — in case of an oracle failure.

The key spending path is a multisig n-of-n that allows parties to collectively agree on spending.

NOTE that if there's no consensus over the pooling transaction, all participants stay with their funds, no further action required. The market will be considered cancelled.

### Phase 3: The market is mature: settling payouts

Once the oracle signs either of the outcomes, the settlement transaction can be built and submitted to the network.

### Phase 4: The market is expired: withdrawing funds

If there was an oracle failure, participants are able to do batch withdrawal (everyone gets back what they put in) after a certain time period.

### Coordination

All participants act in a timely manner according to the timeline specified in the market.  
When it's required to submit a single transaction on behalf of all the participants a simple schedule is applied:
- Participants run PRNG seeded by the txid they want to send
- PRNG defines the order in which participants should try to submit the transaction to the network
- Each participant has a dedicated time slot that lasts N blocks during which it is supposed to send the transaction
- If other participants do not observe the transaction after N blocks, the next one in line takes the lead

## Implementation

The implementation leverages `OP_CSFS` and `OP_CTV` opcodes for external Oracles and spending conditions respectively.

From the UX perspective a user has to do three actions in a timely manner:
- Submit a bet (express intention)
- *idle time period till the market is sealed*
- Broadcast signatures - after the market is sealed but before a certain deadline
- *idle time period till the market is mature*
- Submit the payout transaction (at least one should do that) - after the market is mature, but before a certain deadline

References:
- OP_CSFS - check signature from stack https://bitcoinops.org/en/topics/op_checksigfromstack/
- OP_CTV - check template verify https://bitcoinops.org/en/topics/op_checktemplateverify/
- Payment pool via OP_CTV https://github.com/stutxo/op_ctv_payment_pool
