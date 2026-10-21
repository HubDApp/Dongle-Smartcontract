# Pause Matrix — fine-grained emergency pause (issue #902)

The contract no longer uses a single boolean pause flag. The pause state is a
`PauseState` enum that records **which family of operations is blocked**, so
an incident in one area does not have to take the whole platform down.

```
PauseState::Running                     → nothing is blocked
PauseState::PausedAll                   → every mutating operation is blocked
PauseState::PausedVerification          → only verification operations are blocked
PauseState::PausedFees                 → only fee operations are blocked
PauseState::PausedReviews               → only review operations are blocked
PauseState::PausedVerificationAndFees   → verification and fee operations are blocked
                                         (used by the staged recovery, phase 2)
```

Read operations are **never** blocked — `PauseOperation::Read` always succeeds
so that users can still inspect the state of the platform during an incident.

## Operation families

| `PauseOperation` | Meaning | Typical entry points |
|------------------|---------|----------------------|
| `All` | Generic mutations | `register_project`, `update_project`, `transfer_project`, `link_project`, `set_project_region`, `add_changelog_entry`, collection/bookmark/changelog writes, … |
| `Read` | Read-only calls | `get_project`, `get_collection`, `get_reviews`, `get_contract_status`, … (never blocked) |
| `Verification` | Verification workflow | `request_verification` |
| `Fees` | Fee workflow | `pay_fee`, `cancel_fee_payment` |
| `Reviews` | Review workflow | `add_review`, `add_review_with_attribution`, `submit_review_with_attribution` |

Every mutating entry point calls the guard with the family it belongs to:

```rust
EmergencyPause::require_not_paused(&env, PauseOperation::Reviews)?;
```

## The matrix

| Pause state | `All` | `Read` | `Verification` | `Fees` | `Reviews` |
|-------------|:-----:|:------:|:--------------:|:------:|:---------:|
| `Running` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `PausedAll` | ❌ | ✅ | ❌ | ❌ | ❌ |
| `PausedVerification` | ✅ | ✅ | ❌ | ✅ | ✅ |
| `PausedFees` | ✅ | ✅ | ✅ | ❌ | ✅ |
| `PausedReviews` | ✅ | ✅ | ✅ | ✅ | ❌ |
| `PausedVerificationAndFees` | ✅ | ✅ | ❌ | ❌ | ✅ |

`✅` = the operation succeeds, `❌` = the call fails with
`ContractError::ContractPaused`.

### Staged recovery overrides the generic row

While a staged recovery is in progress (see
[PAUSE_RECOVERY_PHASES.md](PAUSE_RECOVERY_PHASES.md)) the generic `All` family
stays closed until the **final** phase (4 = "fees resumed"), even when the
state itself would allow generic mutations. This keeps the recovery plan
predictable:

| Phase | Pause state | `All` | `Verification` | `Fees` | `Reviews` |
|-------|-------------|:-----:|:--------------:|:------:|:---------:|
| 0 | `PausedAll` | ❌ | ❌ | ❌ | ❌ |
| 1 | `PausedAll` | ❌ | ❌ | ❌ | ❌ |
| 2 | `PausedVerificationAndFees` | ❌ | ❌ | ❌ | ✅ |
| 3 | `PausedFees` | ❌ | ✅ | ❌ | ✅ |
| 4 | `Running` | ✅ | ✅ | ✅ | ✅ |

## API

| Endpoint | Purpose |
|----------|---------|
| `pause(admin)` | Full emergency stop (`PausedAll`), no reason. |
| `set_pause_with_reason(admin, reason, estimated_resume)` | Full emergency stop with a reason and an estimated resume time (issue #901). |
| `set_pause_state(admin, state, reason, estimated_resume)` | Pause / resume a single operation family (issue #902). |
| `unpause(admin)` | Resume everything and clear the pause reason. |
| `get_pause_state()` | Current `PauseState`. |
| `get_contract_status()` | Full status: state, reason, estimated resume, blocked families (issue #901). |
| `is_paused()` | `true` when at least one family is blocked. |

All pause endpoints are admin-only; the pause gate never blocks the admin
recovery calls themselves.

## Operational notes

* Admin-only recovery calls (admin management, fee configuration, verification
  and review moderation, TTL extensions, …) are **never** gated by the pause.
* The legacy boolean flag (`StorageKey::ContractPaused`) is kept in sync with
  the pause record so that `get_config` keeps reporting the pause state.
* `set_pause` (the legacy toggle in the config registry) delegates to the
  emergency-pause module, so the two views can never disagree.
* Every pause transition emits `CONTRACT/PAUSED` (with the reason and the
  estimated resume time) and `CONTRACT/NOTIFY` for the subscribers of the
  pause-notification list (issue #901).
