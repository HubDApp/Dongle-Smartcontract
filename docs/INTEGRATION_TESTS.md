# Integration Test Suite

The bulk of the test suite under `dongle-smartcontract/src/tests/` is unit-level
— one module per registry, asserting a single function's behaviour. This
document catalogs the **integration** tests: those that drive a *sequence* of
public entry points end-to-end, the way a real client would.

## Files

| File | Focus |
|------|-------|
| [`integration_workflows.rs`](../dongle-smartcontract/src/tests/integration_workflows.rs) | 21 cross-cutting multi-step scenarios (this document) |
| [`fee_lifecycle.rs`](../dongle-smartcontract/src/tests/fee_lifecycle.rs) | Full verification-fee payment lifecycle |
| [`verification_lifecycle.rs`](../dongle-smartcontract/src/tests/verification_lifecycle.rs) | register → pay → request → approve → expire → renew |
| [`multisig_and_history.rs`](../dongle-smartcontract/src/tests/multisig_and_history.rs) | Multi-sig proposal approval + admin history |
| [`review_features.rs`](../dongle-smartcontract/src/tests/review_features.rs) | Review tombstones, sorting, cooldown across calls |

## `integration_workflows.rs` scenarios

Run: `cargo test -p dongle-smartcontract integration_workflows`

### Registration → verification → moderation

| # | Test | Sequence |
|---|------|----------|
| 1 | `registration_to_verification_approved` | `register_project` → `request_verification` → `approve_verification`; asserts status `Unverified → Pending → Verified` and `is_verification_active`. |
| 2 | `registration_to_verification_rejected` | register → request → `reject_verification`; asserts project is not left Verified/Pending and `is_verification_active == false`. |
| 3 | `verification_then_revocation` | register → request → approve → `revoke_verification`; asserts status returns to `Unverified`. |
| 4 | `verification_blocked_until_fee_paid` | configure fee token → `request_verification` rejected `InsufficientFee` → `pay_fee` → request succeeds → fee flag consumed. |
| 5 | `archive_then_reactivate_roundtrip` | `archive_project` → double-archive rejected `AlreadyArchived` → `reactivate_project`. |
| 6 | `ownership_transfer_then_new_owner_updates` | `initiate_transfer` → `accept_transfer` → new owner drives `set_reviews_enabled`, old owner now `Unauthorized`. |

### Reviews → moderation

| # | Test | Sequence |
|---|------|----------|
| 7 | `reviews_drive_project_stats` | three `add_review` calls → `get_project_stats` asserts `review_count`, `rating_sum` (×100), `average_rating` (×100). |
| 8 | `review_update_recomputes_average` | `add_review` → `update_review` → average recomputed, count unchanged. |
| 9 | `report_then_hide_then_restore_review` | `add_review` → `report_review` → duplicate report rejected → `hide_review` → double-hide rejected → `restore_review`. |
| 10 | `admin_delete_review_workflow` | `add_review` → `admin_delete_review` → review gone, stats decremented. |
| 11 | `disabling_reviews_blocks_new_reviews` | `set_reviews_enabled(false)` → `add_review` rejected `ReviewsDisabled` → re-enable → succeeds. |
| 12 | `owner_responds_to_review` | `add_review` → `respond_to_review` → `get_review_response` returns the text. |

### Admin governance

| # | Test | Sequence |
|---|------|----------|
| 13 | `add_second_admin_then_remove_first` | `add_admin` → count 2 → `remove_admin` (first) → count 1. |
| 14 | `cannot_remove_last_admin` | `remove_admin` on the sole admin rejected `CannotRemoveLastAdmin`. |
| 15 | `governance_proposal_add_admin_end_to_end` | `create_proposal(AddAdmin)` → `approve_proposal` → `execute_proposal` → candidate is admin. |
| 16 | `pause_blocks_mutations_then_unpause_restores` | `pause` → `register_project` rejected `ContractPaused` → `unpause` → registration works. |

### Fee payment flows

| # | Test | Sequence |
|---|------|----------|
| 17 | `verification_fee_paid_consumed_and_re_paid` | pay → consume via request → approve → revoke → second request rejected `InsufficientFee` → re-pay → succeeds; treasury balance tracked. |
| 18 | `registration_fee_payment_flow` | configure registration fee → `pay_registration_fee` → treasury credited, `get_reg_fee_payment_details` correct. |
| 19 | `fee_config_changes_are_recorded_in_history` | three `set_fee` calls → `get_fee_config_history` has ≥ 3 entries, `get_fee_config` reflects the latest. |

### Curation & social

| # | Test | Sequence |
|---|------|----------|
| 20 | `collection_curation_workflow` | `create_collection` → `add_project_to_collection` ×2 → duplicate add rejected `AlreadyInCollection` → `remove_project_from_collection` → remove-missing rejected `NotInCollection`. |
| 21 | `bookmark_endorse_follow_counters` | `bookmark_project` / `endorse_project` / `follow_project` → counters incremented → idempotency guards (`AlreadyBookmarked`, `AlreadyEndorsed`) → `unendorse_project` decrements. |

## Conventions

- Each test builds a fresh `Env` with `env.mock_all_auths()` and the
  `setup_contract` fixture (contract + one admin).
- Error paths use the generated `try_*` client methods and assert the exact
  `ContractError` variant.
- Fee tests deploy a real Stellar Asset Contract so the token-transfer path is
  exercised and treasury balances are checked.

## Adding a scenario

1. Add a `#[test] fn` to `integration_workflows.rs` (or a dedicated
   `*_lifecycle.rs` file for a large single flow).
2. Drive it exclusively through the public `DongleContractClient` surface.
3. Assert observable state after **each** step, not just at the end.
4. Add a row to the table above.
