# Markstr Core

An interactive protocol for prediction markets on Bitcoin:
- External Oracles via OP_CSFS
- Coinpool via OP_CTV (allows multiple participants)
- Communication via Nostr

## Protocol

### Phase 1: New market is created: accepting bets

Participants signal their intentions to make bets by broadcasting:
- The outcome they want to bet on
- The bet amount
- The payout address

### Phase 2: The market is sealed: pooling funds

- No more bets are accepted at this point (bet timestamp must be less than the deadline).  
- Each participant should have the same view on the market state at this point, otherwise the protocol would fail.
- Participants independently compute the locking script hash and pre-sign the pooling (coinpool) transaction
- Pooling transaction has multiple inputs (one per participant) and a single output protected by OP_CTV
- Once all the signatures are collected, the pooling transaction is submitted to the network

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

## Implementation

The implementation leverages `OP_CSFS` and `OP_CTV` opcodes for external Oracles and spending conditions respectively.

References:
- OP_CSFS - check signature from stack https://bitcoinops.org/en/topics/op_checksigfromstack/
- OP_CTV - check template verify https://bitcoinops.org/en/topics/op_checktemplateverify/
- Payment pool via OP_CTV https://github.com/stutxo/op_ctv_payment_pool
