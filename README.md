# **Swiv Privacy Prediction Protocol**

## 1. Overview

Swiv is a **privacy-preserving, precision-based prediction market** built on Solana using **Anchor** and **MagicBlock Private Ephemeral Rollups**.
Unlike binary prediction markets (Yes/No), Swiv allows users to submit **continuous numerical predictions** (e.g. price levels, values, metrics), producing richer and more accurate market signals.

The protocol separates **private execution** from **public settlement**:

* Sensitive user actions happen inside a **private ephemeral rollup**
* Only finalized, aggregated results are committed to Solana L1

---

## 2. Core Architecture

### 2.1 On-chain Program

* Written in **Rust (Anchor framework)**
* Deployed on Solana
* Manages:

  * Pool lifecycle
  * PDA ownership
  * Vault accounting
  * Reward distribution
  * Protocol fees

### 2.2 Private Execution Layer (MagicBlock)

* Enabled via `#[ephemeral]`
* Handles:

  * Private bet placement
  * Private prediction updates
  * Weight calculations
* Prevents:

  * Front-running
  * Wallet tracking
  * Strategy leakage

### 2.3 Public Settlement Layer

* Final states are flushed back to Solana via:

  * Pool undelegation
  * Bet undelegation
* Enables permissionless reward claiming

---

## 3. Protocol Lifecycle (End-to-End Flow)

### 3.1 Protocol Initialization (Admin)

```rust
initialize_protocol(protocol_fee_bps)
```

* Creates the global protocol config
* Sets:

  * Admin authority
  * Treasury wallet
  * Protocol fee (basis points)
* Executed **once**

---

### 3.2 Pool Creation

```rust
create_pool(
  pool_id,
  name,
  metadata,
  start_time,
  end_time,
  max_accuracy_buffer,
  conviction_bonus_bps
)
```

Each pool defines:

* A prediction window (`start_time → end_time`)
* A numerical outcome range
* Accuracy tolerance (`max_accuracy_buffer`)
* Conviction incentives

The pool PDA:

* Owns a token vault
* Tracks total stake, weights, and resolution state

---

### 3.3 Pool Delegation to TEE (Privacy Activation)

```rust
delegate_pool(pool_id)
```

* Transfers pool PDA authority to MagicBlock TEE
* From this point:

  * Pool state becomes **privately mutable**
  * No public on-chain writes for user actions

This step is **required before resolution**.

---

### 3.4 Private Bet Lifecycle (Users)

#### 3.4.1 Initialize Bet

```rust
init_bet(amount, request_id)
```

* Creates a bet PDA
* Locks user stake into pool vault
* Stores encrypted metadata in the rollup

#### 3.4.2 Place / Update Prediction

```rust
place_bet(prediction, request_id)
update_bet(new_prediction)
```

* Prediction value remains private
* Users may update predictions before pool expiry
* Update count affects conviction bonus

---

## 4. Pool Resolution & Settlement

### 4.1 Resolve Pool (TEE-only)

```rust
resolve_pool(final_outcome)
```

* Called **inside MagicBlock TEE**
* Marks pool as resolved
* Stores the final numerical outcome
* Enables weight computation

No user data is revealed on-chain at this stage.

---

### 4.2 Weight Calculation (Batch, Private)

```rust
batch_calculate_weights()
```

* Admin passes all **bet PDAs** as remaining accounts
* Each bet weight is calculated privately
* Results are written into bet accounts

This avoids per-user transactions and preserves privacy.

---

## 5. Weight Calculation Model (Core Math)

Swiv’s reward system is **stake-weighted and accuracy-driven**.

### 5.1 Accuracy Score

```rust
calculate_accuracy_score(prediction, result, buffer)
```

* Measures closeness to final outcome
* Linear decay inside buffer
* Outside buffer → score = 0

Formula:

```
accuracy = 1 - (|prediction - result| / buffer)
```

Scaled by `MATH_PRECISION = 1_000_000`

---

### 5.2 Time Bonus

```rust
calculate_time_bonus(start, end, entry_time)
```

* Rewards earlier participation
* Longer commitment → higher multiplier

Earlier bets receive higher influence.

---

### 5.3 Conviction Bonus

```rust
calculate_conviction_bonus(update_count)
```

* No updates → higher conviction bonus
* Multiple updates → neutral weight

Encourages confidence, not constant adjustment.

---

### 5.4 Final Weight Formula

```rust
weight =
stake
× accuracy
× time_bonus
× conviction
```

Scaled down by precision constants:

```rust
final_weight = raw_product / P³
```

This produces a fair, manipulation-resistant influence score.

---

## 6. Finalization & Public Settlement

### 6.1 Finalize Weights (On-chain)

```rust
finalize_weights()
```

Requirements:

* Pool must be resolved
* Weights must not be finalized already

Actions:

* Deduct protocol fee
* Lock distributable vault balance
* Emit final settlement event

Once called:

* Pool becomes immutable
* Claiming is enabled

---

### 6.2 Flush State Back to L1

```rust
batch_undelegate_bets()
undelegate_pool()
```

* Writes finalized bet + pool data to Solana
* Ends private execution phase

---

## 7. Reward Claiming (Users)

```rust
claim_reward()
```

* Permissionless
* Based on:

  ```
  user_weight / total_pool_weight
  ```
* Transfers tokens directly from pool vault

No admin trust required.

---

## 8. Emergency Handling

```rust
emergency_refund()
```

* Used if pool cannot be resolved
* Returns stakes proportionally
* Prevents fund lockups

---

## 9. Key Guarantees

* **Privacy**: Predictions never hit public mempool
* **Fairness**: No front-running or copy trading
* **Precision**: Not limited to binary outcomes
* **Verifiability**: Final state is fully on-chain
* **Scalability**: Batch operations reduce gas costs

---

## 10. Summary

Swiv introduces a new category of prediction markets by combining:

* Precision-based numerical forecasting
* Private ephemeral execution
* Trustless on-chain settlement

By leveraging **MagicBlock Private Ephemeral Rollups** and **Solana’s PDA model**, Swiv delivers privacy without sacrificing decentralization or verifiability.