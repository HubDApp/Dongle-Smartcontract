# Emergency Pause & Recovery

Operational reference for the Dongle smart contract's emergency pause: the
pause/unpause state machine, an incident recovery checklist for the operations
team, and the state validation to run after unpausing.

Audience: contract admins and incident responders. Pair this with
[ADMIN_ROTATION_PLAYBOOK.md](ADMIN_ROTATION_PLAYBOOK.md) (key custody, admin set)
and [THREAT_MODEL.md](THREAT_MODEL.md) (what the pause is meant to contain).

---

## 1. What the pause is

The emergency pause is a single on-chain boolean, `StorageKey::ContractPaused`,
toggled by admin-only entry points and enforced as a pre-condition on a defined
set of mutating calls. It is implemented in `src/emergency_pause.rs`
(`EmergencyPause`) and exposed through three contract functions:

| Function | Auth | Effect |
|---|---|---|
| `pause(admin)` | admin | Sets `ContractPaused = true`. Emits `CONTRACT/PAUSED`. Idempotent. |
| `unpause(admin)` | admin | Sets `ContractPaused = false`. Emits `CONTRACT/UNPAUSED`. Idempotent. |
| `is_paused() -> bool` | none (read) | Returns the current flag. Defaults to `false` if never set. |

The pause **only flips a flag**. It does not snapshot, migrate, or roll back any
stored data. Unpausing is therefore a pure no-op with respect to every record;
this is covered by the tests in
`src/tests/pause_state_recovery.rs` (see [§5](#5-state-validation-after-unpause)).

### 1b. Audit trail asymmetry

`pause()` / `unpause()` emit the `CONTRACT/PAUSED` / `CONTRACT/UNPAUSED` events
but do **not** write an `AdminActionLog` entry. The transaction and its event are
the audit record for an emergency stop. (The unenforced `set_pause` path *does*
write `ContractPaused` / `ContractResumed` admin-log entries — another reason not
to conflate the two.)

### 1a. Two pause flags — do not confuse them

There are two independent "paused" concepts in storage:

| Flag | Set by | Read by | Enforced? |
|---|---|---|---|
| `StorageKey::ContractPaused` | `pause()` / `unpause()` | `is_paused()` | **Yes** — this is the emergency stop. |
| `ExtensionKey::Paused` | `set_pause(admin, bool)` | `get_config().paused` | **No** — advisory metadata only; reserved for a future enforcement change. |

Incident response uses **`pause()` / `unpause()` / `is_paused()`** exclusively.
`set_pause` changes a field in `get_config` but does **not** block any call
today. Do not rely on it during an incident.

---

## 2. Pause / unpause state machine

```
                 pause(admin)                     pause(admin)  → no-op (stays PAUSED, re-emits event)
        ┌──────────────────────────────┐          unpause(admin)→ no-op (stays RUNNING, re-emits event)
        ▼                              │
  ┌───────────┐   pause(admin)   ┌───────────┐
  │  RUNNING  │ ───────────────▶ │  PAUSED   │
  │ (default) │ ◀─────────────── │           │
  └───────────┘  unpause(admin)  └───────────┘
        │                              │
        │ gated mutating calls: OK     │ gated mutating calls: Err(ContractPaused = 61)
        │ reads: OK                    │ reads: OK
        │ admin-recovery calls: OK     │ admin-recovery calls: OK
```

State transitions:

| From | Call | To | Event | Notes |
|---|---|---|---|---|
| RUNNING | `pause(admin)` | PAUSED | `CONTRACT/PAUSED` | |
| PAUSED | `pause(admin)` | PAUSED | `CONTRACT/PAUSED` | Idempotent; still succeeds, re-emits. |
| PAUSED | `unpause(admin)` | RUNNING | `CONTRACT/UNPAUSED` | |
| RUNNING | `unpause(admin)` | RUNNING | `CONTRACT/UNPAUSED` | Idempotent; still succeeds, re-emits. |
| either | `pause`/`unpause` by non-admin | unchanged | none | `Err(AdminOnly)`. |

Because `pause`/`unpause` are idempotent and re-emit their event, indexers should
treat `CONTRACT/PAUSED` / `CONTRACT/UNPAUSED` as **level** signals (reconcile
against `is_paused()`), not as edge-triggered counters.

---

## 3. What the pause blocks (enforcement surface)

While `is_paused()` is `true`, exactly these entry points return
`ContractError::ContractPaused` (code `61`) before doing any work:

| Domain | Blocked while paused |
|---|---|
| Project lifecycle | `register_project`, `update_project`, `set_project_lifecycle_status`, `set_project_region` |
| Project security contact | `update_security_contact`, `submit_security_contact_proof` |
| Project links | `link_project`, `unlink_project` |
| Ownership transfer | `initiate_transfer`, `cancel_transfer` |
| Fees | `cancel_fee_payment` |
| Changelog | `add_changelog_entry`, `remove_changelog_entry` |

**Always allowed (not gated):**

- All read/getter calls (`get_project`, `list_*`, `get_project_stats`,
  `get_config`, `is_paused`, …).
- All admin-only recovery calls: `pause`, `unpause`, `add_admin`,
  `remove_admin`, timelock and multisig governance, `set_fee` / fee config,
  verification approve / reject / revoke / assign, review moderation
  (hide / restore / admin-delete), TTL / storage extension, `set_featured`,
  collection management, dispute and report resolution.

### 3a. Known limitation — enforcement coverage is partial

The gate is currently applied only to the calls listed above. Some non-admin
mutating paths — including `pay_fee`, `add_review` / `submit_review`,
`request_verification` / renewal, `follow_project`, `bookmark_project`,
`endorse_project`, and `archive_project` / `reactivate_project` — are **not**
gated and remain callable while paused.

Operational implication: an emergency pause stops project registration and
mutation, transfers, and changelog writes, but does **not** fully freeze the
contract. If an incident requires blocking one of the ungated paths, the
available levers are removing the relevant config (e.g. setting fees so a path
errors), verification/review moderation, or an admin-side mitigation specific to
the incident. Widening the gate is tracked separately from this document.

---

## 4. Incident recovery checklist (operations team)

### Phase 0 — Before you pause

- [ ] Confirm you are authenticated as a current admin: `is_admin(<your addr>)`
      returns `true`.
- [ ] Confirm at least one **other** admin key is available (`get_admin_list`) so
      recovery is not single-key dependent — see
      [ADMIN_ROTATION_PLAYBOOK.md](ADMIN_ROTATION_PLAYBOOK.md).
- [ ] Record the trigger: what was observed, which tx / address / project,
      timestamp, and who is running point.
- [ ] Note the pre-incident state you will validate against later:
      `get_project_count`, `get_action_log_count`, `get_config`.

### Phase 1 — Pause

- [ ] Call `pause(admin)`.
- [ ] Verify `is_paused()` returns `true`.
- [ ] Verify the `CONTRACT/PAUSED` event was emitted in the tx result. Note:
      `pause()` does **not** write an `AdminActionLog` entry — record the tx hash
      yourself as the audit anchor (see [§1b](#1b-audit-trail-asymmetry)).
- [ ] Announce the pause to stakeholders with the trigger summary.

### Phase 2 — Investigate & remediate

- [ ] Reproduce / confirm the issue against a fork or read-only queries.
- [ ] Apply the fix using admin-recovery calls that bypass the pause
      (fee config, verification / review moderation, admin set changes,
      timelock/multisig governance, TTL extension).
- [ ] For anything the pause does **not** cover (see [§3a](#3a-known-limitation--enforcement-coverage-is-partial)),
      decide and document the mitigation explicitly.
- [ ] Keep a running log of every admin call made during the incident (tx hash,
      function, args, result).

### Phase 3 — Pre-unpause validation

- [ ] Run the full state validation in [§5](#5-state-validation-after-unpause)
      **while still paused**. Reads work while paused, so do this before lifting
      the stop.
- [ ] Confirm every value matches the pre-incident baseline from Phase 0, or that
      each difference is an intended remediation with a corresponding admin-log
      entry.
- [ ] Get a second admin to independently review the validation output.

### Phase 4 — Unpause

- [ ] Call `unpause(admin)`.
- [ ] Verify `is_paused()` returns `false`.
- [ ] Verify the `CONTRACT/UNPAUSED` event in the tx result (again, no
      `AdminActionLog` entry is written — keep the tx hash).
- [ ] Smoke-test one previously-blocked call end to end (e.g. `register_project`
      with a throwaway project, then `update_project`, then leave it or archive
      it per policy).
- [ ] Re-run [§5](#5-state-validation-after-unpause) once more — the numbers must
      be identical to the paused readings plus only your smoke-test delta.
- [ ] Announce resolution. Attach the incident log and validation output.

### Phase 5 — Post-incident

- [ ] Write the post-mortem: timeline, root cause, why the pause was / wasn't
      sufficient, follow-up work.
- [ ] File issues for any enforcement gap or tooling gap the incident exposed.
- [ ] If admin keys were exposed, run the rotation playbook.

---

## 5. State validation after unpause

The pause flag is orthogonal to all stored data, so a pause/unpause cycle must
leave every record byte-for-byte unchanged. Validate by comparing these reads
against the pre-incident baseline (and, ideally, capturing them once while paused
and again right after unpause — the two must match):

| Check | Call | Expectation |
|---|---|---|
| Pause flag cleared | `is_paused()` | `false` |
| Project count | `get_project_count()` | Unchanged vs. baseline (± intended remediation) |
| Spot-check projects | `get_project(id)` for a sample of ids | Identical struct vs. baseline |
| Project listing | `list_projects(1, N)` | Same ids, same order |
| Reviews | `list_reviews(pid, 0, N)`, `get_project_stats(pid)` | Identical review set and `rating_sum` / `review_count` / `average_rating` |
| Featured list | `list_featured_projects(0, N)` | Same ids, same order |
| Admin set | `get_admin_list()` | Unchanged (unless rotation was part of remediation) |
| Fee config | `get_fee_config()` | Matches baseline or the intended new config |
| Audit trail | `get_action_log_count()`, `list_admin_actions(0, K)` | Baseline count **+** one entry per remediation admin call, and nothing else. `pause` / `unpause` themselves add **no** admin-log entries (see [§1b](#1b-audit-trail-asymmetry)); reconcile them against the `CONTRACT/PAUSED` / `CONTRACT/UNPAUSED` events and tx hashes instead. |
| Config view | `get_config()` | Limits/version unchanged; `paused` (advisory field) reflects only explicit `set_pause` calls, if any |

If any check fails to reconcile, **do not consider the incident closed** — treat
the discrepancy as a new incident.

### Automated coverage

`src/tests/pause_state_recovery.rs` asserts the data-integrity guarantee:

- `pause_then_unpause_returns_to_operational_state` — flag returns to `false`.
- `pause_unpause_preserves_project_data` — projects and count unchanged.
- `pause_unpause_preserves_reviews_and_stats` — reviews and aggregates unchanged.
- `pause_unpause_preserves_featured_and_admin_state` — curation and admin set
  unchanged.
- `repeated_pause_unpause_cycles_preserve_data_integrity` — five cycles are a
  stable no-op; reads while paused return identical data; the contract is fully
  operational afterwards.
- `admin_recovery_writes_during_pause_persist_after_unpause` — a write made while
  paused is persisted, not rolled back by `unpause`.

Run them with `cargo test -p dongle-contract pause_state_recovery`.
