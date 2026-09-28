# Approval Threshold Consistency Audit

**Scope:** Uniformity of admin approval-threshold (multi-sig quorum)
enforcement across every proposal type in the admin governance module.

**Date:** 2026-08-27
**Audited source:** `dongle-smartcontract/src/admin_manager.rs`,
`dongle-smartcontract/src/timelock_manager.rs`, `constants.rs`, `errors.rs`.
**Related issue:** #630.

---

## 1. What the threshold is

`AdminApprovalThreshold` (storage: `ExtensionKey::AdminApprovalThreshold`,
default `1`) is the number of distinct admin approvals a proposal must collect
before it can execute. `get_admin_approval_threshold` returns it; it is read
**live** on every call and never snapshotted into a proposal.

## 2. Proposal types

`ProposalPayload` has 7 variants, all routed through the same
`create_proposal` / `approve_proposal` / `execute_proposal` pipeline:

| Variant | Effect on execute |
| --- | --- |
| `AddAdmin(Address)` | Add an admin |
| `RemoveAdmin(Address)` | Remove an admin (not the last one) |
| `SetFee(token, verification_fee, registration_fee, treasury)` | Replace fee config + treasury |
| `SetThreshold(u32)` | Change the approval threshold |
| `ApproveVerification(project_id)` | Mark a project Verified |
| `RejectVerification(project_id)` | Mark a project Rejected |
| `RevokeVerification(project_id, reason)` | Revoke a Verified project |

## 3. Where the threshold is enforced

### 3.1 Status assignment — `create_proposal` / `approve_proposal`

Both compute `status = Approved` iff `approvals.len() >= threshold`, using the
live threshold, **independent of the payload variant**. `create_proposal`
seeds `approvals` with the proposer; `approve_proposal` adds one distinct
admin per call, rejects duplicates, and only operates on `Pending` proposals.

### 3.2 Execution gate — `execute_proposal`

```
if proposal.status != ProposalStatus::Approved      -> InvalidStatus
let threshold = get_admin_approval_threshold(env)
if proposal.approvals.len() < threshold             -> Unauthorized
match proposal.payload { /* 7 arms */ }
proposal.status = Executed
```

**Finding (positive):** the `approvals.len() < threshold` check sits *before*
the `match` on the payload, so **all 7 payload variants are gated by exactly
the same, live-re-evaluated quorum check.** There is no per-variant bypass and
no variant with a weaker check. The pre-`match` guard also re-validates the
`Approved` status and the payload hash, and rejects expired proposals — again,
uniformly for every variant.

### 3.3 Extra rule for `SetThreshold` downgrades

`SetThreshold` carries **one additional** constraint on top of the shared
gate, evaluated inside its match arm:

- `new_threshold == 0 || new_threshold > admin_count` → `InvalidProjectData`.
- If `new_threshold < current_threshold` **and**
  `approvals.len() <= current_threshold` →
  `ThresholdDowngradeRequiresSupermajority` (code 74).

This is a deliberate *tightening*, not an inconsistency: lowering the quorum
requires strictly more approvals than the quorum being dismantled, so a
group of exactly `current_threshold` colluding admins cannot quietly reduce
future quorum. Threshold *increases* and no-ops add no extra requirement.

## 4. Direct (non-proposal) governance paths

| Function | Guard | Behaviour when threshold > 1 |
| --- | --- | --- |
| `set_admin_approval_threshold` | `if current_threshold > 1 { Unauthorized }` | Blocked — bootstrap-only path; all later changes go through `SetThreshold` proposals. Also validates `threshold in 1..=admin_count`. |
| `add_admin` | `if threshold > 1 { MultiSigRequired }` | Blocked — must use `AddAdmin` proposal. |
| `remove_admin` | `if threshold > 1 { MultiSigRequired }` | Blocked — must use `RemoveAdmin` proposal. |

**Finding (positive):** once multi-sig is active there is **no** direct path
that mutates the admin set, the fee config, the threshold, or verification
state without going through the quorum-gated proposal pipeline.

## 5. Findings

| # | Severity | Finding | Status |
| --- | --- | --- | --- |
| F-1 | Info | Threshold enforcement in `execute_proposal` is uniform across all 7 `ProposalPayload` variants (single pre-`match` guard). | ✅ No change needed |
| F-2 | Info | `create_proposal` and `approve_proposal` compute `Approved` status uniformly and from the live threshold. | ✅ No change needed |
| F-3 | Info | `SetThreshold` downgrade supermajority rule is an intentional tightening, correctly scoped to reductions only. | ✅ Documented (code 74, CHANGELOG) |
| F-4 | **Low / build-blocking** | `admin_manager.rs` returns `ContractError::MultiSigRequired` from `add_admin` / `remove_admin`, but that variant **did not exist** in `errors.rs` — a compile error and part of the currently-broken build (see `BUILD_STATUS.md`). | ✅ **Fixed in this PR** — added `MultiSigRequired` (code 77) |
| F-5 | Info | `reject_proposal` performs no threshold check: any single admin can move a `Pending` proposal to `Rejected`. This is intentional — rejection blocks a change rather than enacting one — and matches a "any guardian can veto" model. | ✅ No change needed; documented here |
| F-6 | Info | The threshold is not snapshotted. Raising it can strand an already-`Approved` proposal (no more approvals can be added because `approve_proposal` only accepts `Pending`). Lowering it lets a `Pending` proposal execute on its existing approvals. | ✅ Already documented in `CONTRACT_INTERFACE.md` §"Threshold changes and existing proposals"; operators should drain the queue before raising the threshold. |
| F-7 | Info | `execute_proposal` re-checks the threshold even for proposals already marked `Approved` (defence in depth against a threshold raised after approval). | ✅ No change needed |

## 6. Threshold-downgrade exception handling — reference

| Scenario | Required approvals to execute `SetThreshold(new)` |
| --- | --- |
| `new > current` (increase) | `current` (the normal gate) |
| `new == current` (no-op) | `current` |
| `new < current` (downgrade) | `current + 1` (strictly more than `current`) |
| `new == 0` or `new > admin_count` | rejected regardless (`InvalidProjectData`) |

If a downgrade proposal cannot reach `current + 1` approvals it can never
execute; create a new proposal once more admins are available, or raise the
threshold in smaller steps is **not** possible (only `SetThreshold` changes
it) — plan downgrades when enough admins are online.

## 7. Test matrix (for a follow-up test PR)

Not implemented in this PR (tests are out of scope per the issue's working
constraints). A future `src/tests/approval_threshold.rs` should cover, for
`threshold = 3`, `admins = 4`:

- Each of the 7 payload types: 2 approvals → `execute_proposal` returns
  `Unauthorized`; 3 approvals → succeeds.
- `create_proposal` returns `Pending` (1 approval < 3).
- Raising threshold to 4 after a proposal reached 3 approvals blocks its
  execution with `Unauthorized`.
- `SetThreshold(2)` with exactly 3 approvals →
  `ThresholdDowngradeRequiresSupermajority`; with 4 approvals → succeeds.
- `SetThreshold(1)` downgrade path likewise needs 4 approvals.
- `add_admin` / `remove_admin` / `set_admin_approval_threshold` direct calls
  return `MultiSigRequired` / `Unauthorized` while threshold > 1.
- `reject_proposal` succeeds with a single admin regardless of threshold.

## 8. Conclusion

Approval-threshold validation **is uniform** across all proposal types: the
enforcement point is a single guard shared by every payload variant, evaluated
against the live threshold, with only an intentional extra constraint on
threshold downgrades. The one real defect found (F-4, the missing
`MultiSigRequired` variant) is fixed in this PR. No non-uniform or bypassable
threshold check was found.
