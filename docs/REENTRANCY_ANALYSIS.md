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

## State consistency when the external call itself fails (#693)

Everything above concerns re-entrancy specifically. A separate question this
document didn't originally cover: what happens to *this contract's own
storage* when the token transfer at one of the three sites above simply
fails — insufficient balance, a frozen trustline, an untrusted/misconfigured
token address, etc. — rather than being re-entered?

The answer follows directly from Soroban's invocation model already
described above: a panic/trap anywhere inside a single top-level invocation
(including inside a cross-contract sub-invocation) aborts that entire
invocation, and the host rolls back *every* storage write made during it —
not just the failed sub-call. So for all three sites:

- `execute_fee_payment`: the transfer runs *before* the paid flag/payment
  record are written, so a failed transfer never even reaches those writes.
- `claim_fee_refund` / `cancel_fee_payment`: the state write runs *before*
  the transfer (CEI, as defence-in-depth against a hypothetical future
  re-entrancy), but if the transfer then fails, the *whole* invocation rolls
  back — including that earlier write — so it's not observable either way.

`dongle-smartcontract/src/tests/cross_contract_call_safety_693.rs` (issue
#693) is the empirical check for this: each of the three sites is driven
through an actual transfer failure (zero balance for `pay_fee`, a drained
treasury for `claim_fee_refund`/`cancel_fee_payment`) and asserts no partial
state survives — both via the contract's own getters and a raw storage
snapshot taken before/after.

**External contract assumption, stated explicitly:** all three sites trust
that the configured token conforms to the standard interface
(`soroban_sdk::token::Client`) and that `transfer` either fully succeeds or
traps without partially applying. Nothing in this contract can protect
against a *malicious* token contract that reports success without moving
funds — that trust boundary is set by whoever configures `token` via
`set_fee`, not enforced by this contract.

---

## References

- [Soroban security model](https://developers.stellar.org/docs/build/smart-contracts/security)
- [SEP-0041 Token Interface](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)
- Issue #621: Reentrancy Protection Missing
- Issue #693: Cross-Contract Call Safety and State Consistency
