# Reentrancy Analysis — Dongle Smart Contract (#621)

## Summary

Traditional EVM-style reentrancy attacks are **not possible** in this contract
under the current Soroban host. Soroban's execution model eliminates the attack
surface at the platform level.

---

## Soroban's Re-entrancy Model

| Property | EVM | Soroban |
|---|---|---|
| Execution model | Stack-based, callbacks possible | WASM sandbox, no mid-execution callbacks |
| Cross-contract calls | Synchronous _and_ re-entrant via fallback/receive | Synchronous sub-invocations; caller is suspended |
| Self-re-entrancy | Possible if not guarded | Rejected by host — calling an already-active contract is a host error |
| Gas/resource limit | Per-transaction gas | Per-invocation compute units; no shared mutable global |

Soroban's host explicitly prevents a contract from appearing more than once on
the call stack. Any attempt to invoke a contract that is already executing
results in a host-level trap before reaching contract code, so no reentrancy
guard in contract code is required or useful.

---

## Cross-Contract Call Sites

The only outbound cross-contract calls in this contract are token transfers
against a Stellar Asset Contract (SAC) or compatible token:

### 1. `execute_fee_payment` — payer → treasury

```rust
// In fee_manager.rs
client.transfer(&payer, &treasury, &(amount as i128));
// … state flags written after the call …
env.storage().persistent().set(&paid_flag_key, &true);
```

The paid flag is written **after** the transfer. Under the EVM this ordering
would be exploitable via a re-entrant fallback. Under Soroban it is safe
because the token contract cannot call back into this contract during the
transfer. The ordering is noted here for future review if Soroban ever
introduces asynchronous messaging.

### 2. `claim_fee_refund` — treasury → payer

```rust
// checks-effects-interactions pattern applied explicitly
refund.claimed_at = Some(env.ledger().timestamp());  // ← effect written first
env.storage().persistent().set(&ExtensionKey::FeeRefund(project_id), &refund);
// … then the interaction …
token_client.transfer(&treasury, &refund.payer, &(refund.amount as i128));
```

The `claimed_at` timestamp is persisted **before** the transfer. This follows
the checks-effects-interactions (CEI) pattern as defence-in-depth.

### 3. `cancel_fee_payment` — treasury → payer

```rust
// Effect: remove the paid flag from storage first
env.storage().persistent().remove(&StorageKey::FeePaidForProject(project_id));
env.storage().persistent().remove(&ExtensionKey::FeePaymentDetails(project_id));
// Interaction: then transfer tokens
token_client.transfer(&treasury, &record.payer, &(record.amount as i128));
```

The storage flags are removed **before** the transfer, consistent with CEI.

---

## Verdict

| Risk | Present? | Mitigation |
|---|---|---|
| Classic re-entrancy (same-contract callback) | ❌ No | Soroban host rejects self-re-entrancy |
| Cross-contract callback loop | ❌ No | Soroban has no callback/fallback mechanism |
| Ordering-based double-spend (`claim_fee_refund`) | ❌ No | `claimed_at` written before transfer |
| Ordering-based double-spend (`cancel_fee_payment`) | ❌ No | Storage flags removed before transfer |
| Ordering-based double-spend (`execute_fee_payment`) | ⚠️ Theoretical | Safe today; annotated for future review |

**No reentrancy protection code is needed.** This analysis should be revisited
if the Soroban host introduces:
- Asynchronous cross-contract messaging
- Callback / delegate-call semantics
- Mutable shared global state across invocations

---

## References

- [Soroban security model](https://developers.stellar.org/docs/build/smart-contracts/security)
- [SEP-0041 Token Interface](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)
- Issue #621: Reentrancy Protection Missing
