# Test Coverage

This document is the central coverage reference for the `dongle-contract`
crate: how to generate a coverage report, the target thresholds, which
contract entry points are covered by which tests, and the known gaps.

It pairs with [`TEST_ORGANIZATION.md`](TEST_ORGANIZATION.md) (which owns the
*file layout* of the suite) and [`CI_CD.md`](CI_CD.md) (which owns the
pipeline). This file owns the **function → test** mapping and the coverage
number.

---

## 1. Generating a coverage report

Coverage is measured on the **native host target** (the same target
`cargo test` uses — Soroban `testutils` is host-only).

### 1.1 `cargo tarpaulin` (primary)

```bash
cd dongle-smartcontract
cargo install cargo-tarpaulin        # once

# Human-readable + machine-readable, written to ../coverage/
cargo tarpaulin \
  --lib \
  --engine llvm \
  --out Html --out Xml --out Lcov \
  --output-dir ../coverage \
  --exclude-files 'src/tests/*' \
  --timeout 300
```

* `--lib` runs the `#[cfg(test)] mod tests` tree (all 55 active modules).
* `--exclude-files 'src/tests/*'` keeps test code out of the denominator — we
  measure coverage *of the contract*, not of the tests.
* `--engine llvm` is more accurate for `#[no_std]` + macro-heavy Soroban code
  than the default ptrace engine.
* Open `../coverage/tarpaulin-report.html`; `../coverage/cobertura.xml` and
  `../coverage/lcov.info` are for CI upload / editor gutters.

### 1.2 `cargo llvm-cov` (alternative)

```bash
cd dongle-smartcontract
cargo install cargo-llvm-cov
cargo llvm-cov --lib --ignore-filename-regex 'src/tests/' --html --lcov --output-path ../coverage/lcov.info
```

Use one tool consistently for trend tracking; llvm-cov and tarpaulin report
slightly different numbers because of how they attribute macro-expanded lines.

### 1.3 Per-function view

```bash
cargo llvm-cov --lib --ignore-filename-regex 'src/tests/' --show-missing-lines
# or, region/function summary:
cargo llvm-cov report --lib --ignore-filename-regex 'src/tests/'
```

---

## 2. Targets

| Scope | Target | Rationale |
|-------|-------:|-----------|
| Overall line coverage (contract code, `src/tests/` excluded) | **≥ 85 %** | Acceptance criterion for issue #615 |
| Critical paths (see §3) | **100 %** line + branch | These move funds, gate authorization, or govern the admin set — an untested branch here is a direct security risk |
| New / changed code in a PR | **≥ 85 %** patch coverage | Prevents slow erosion |
| Getters / view functions | best-effort | Low risk; covered incidentally by the flows that assert on them |

The **85 % figure is a target, not yet a measured-and-enforced gate.**
`ci.yml` does not currently run a coverage job (see §6 Roadmap). Until it does,
run §1 locally before a release and record the number in §5.

---

## 3. Critical paths (must be 100 %)

These entry points and their error branches require complete coverage. Each row
notes the primary test file(s) that must exercise every branch.

| Area | Entry points | Primary tests | Branch-coverage focus |
|------|--------------|---------------|-----------------------|
| Contract init | `initialize` | `admin`, `fixtures`, `typed_error_regressions` | double-init rejected (`AlreadyInitialized`) |
| Admin set | `add_admin`, `remove_admin`, `set_admin_approval_threshold` | `admin`, `invariants`, `proposal_threshold` | last-admin protection, threshold ≥ count, supermajority downgrade rule |
| Multisig governance | `create_proposal`, `approve_proposal`, `reject_proposal`, `execute_proposal` | `multisig_and_history`, `proposal_threshold` | threshold not met, double-approve, expired proposal, execute-before-threshold |
| Timelock | `schedule_set_fee`, `schedule_add_admin`, `schedule_remove_admin`, `execute_scheduled_*`, `cancel_scheduled_action` | `timelock` | not-expired, already-executed, already-cancelled, not-found |
| Fees / value transfer | `set_fee`, `pay_fee`, `pay_registration_fee`, `cancel_fee_payment`, `claim_fee_refund`, `get_fee_refund` | `fee_lifecycle`, `fee_refund`, `fee_boundary`, `fee_token_rotation` | wrong amount, wrong token, double-pay, refund double-claim, zero-fee path |
| Verification lifecycle | `request_verification`, `approve_verification`, `reject_verification`, `revoke_verification`, `renew_verification` | `verification`, `verification_lifecycle`, `verification_replacement`, `renewal` | fee-not-paid gate, invalid state transitions, re-request after reject/revoke |
| Emergency pause | `pause`, `unpause`, `set_pause` | `emergency_pause` **(currently orphaned — see §4)** | every mutating endpoint rejected while paused |
| Ownership transfer | `initiate_transfer`, `accept_transfer`, `cancel_transfer` | `transfer` | non-owner initiate, accept by wrong account, cancel after accept |
| Verified-field freeze | `update_project` while `Verified` | `verified_freeze` **(currently disabled — see §4)** | each frozen field rejected; thaw after revoke |
| Panic-as-DoS guards | all `-> Result` entry points | `typed_error_regressions`, `proptest_pagination` | see [`THREAT_MODEL.md`](THREAT_MODEL.md) §4.5 P2 |

> Two critical-path areas — **emergency pause** and **verified-field freeze** —
> have full test files (`emergency_pause.rs`, 23 tests; `verified_freeze.rs`,
> 19 tests) that are **not compiled** because they are not wired into
> `src/tests/mod.rs`. Re-enabling them (tracked in `TEST_ORGANIZATION.md`
> §2.2/§2.3) is a prerequisite for claiming 100 % critical-path coverage.

---

## 4. Coverage map — entry point → test file

`dongle-contract` exposes ~198 public entry points in `src/lib.rs`. The table
below maps each domain to its covering test files. Files in **_italics_** are
**not currently compiled** (orphaned or disabled per
[`TEST_ORGANIZATION.md`](TEST_ORGANIZATION.md)) — coverage that depends only on
an italic file does not count until that file is re-enabled.

Regenerate this mapping:

```bash
cd dongle-smartcontract
for fn in $(grep -oE 'pub fn [a-z_0-9]+' src/lib.rs | sed 's/pub fn //' | sort -u); do
  printf '%-34s %s\n' "$fn" "$(grep -rl "\b$fn\b" src/tests/ | xargs -n1 basename | paste -sd, -)"
done
```

### Admin, governance, timelock

| Entry points | Covering tests |
|--------------|----------------|
| `initialize`, `is_admin`, `add_admin`, `remove_admin`, `get_admin_list`, `get_admin_count` | `admin`, `invariants`, `multisig_and_history`, `proposal_threshold`, `timelock`, `admin_action_log`, _auth_matrix_, _authorization_, _emergency_pause_ |
| `create_proposal`, `approve_proposal`, `reject_proposal`, `execute_proposal`, `get_proposal`, `list_proposals` | `multisig_and_history`, `proposal_threshold` |
| `get_admin_approval_threshold`, `set_admin_approval_threshold` | `multisig_and_history`, `proposal_threshold` |
| `schedule_set_fee`, `schedule_add_admin`, `schedule_remove_admin`, `execute_scheduled_set_fee`, `execute_scheduled_add_admin`, `execute_scheduled_remove_admin`, `cancel_scheduled_action`, `get_scheduled_action`, `list_scheduled_actions`, `get_scheduled_action_count` | `timelock` |
| `get_admin_action_log_entry`, `list_admin_actions`, `get_admin_action_log_count` | `admin_action_log`, `typed_error_regressions` |
| `get_admin_action_log_by_admin` | **none — GAP** |
| `pause`, `unpause` | `changelog`, _emergency_pause_ |
| `set_pause`, `is_paused` | `is_paused`: _emergency_pause_ only; `set_pause`: **none — GAP** |

### Projects & metadata

| Entry points | Covering tests |
|--------------|----------------|
| `register_project` | `registration`_, _slug_, `field_limits`, `tags`, `fixtures`, `verification`, `transfer`, `region_and_integrity`, `license_metadata`, `launch_timestamp`_, `pagination`_, +~20 more |
| `update_project` | `field_limits`, `tags`, `tag_index`, `license_metadata`, `region_and_integrity`, _verified_freeze_, _auth_matrix_, _authorization_, _atomicity_, _pagination_, _reserved_names_ |
| `get_project`, `get_project_by_slug`, `list_projects`, `get_project_count`, `get_projects_by_owner`, `get_owner_project_count` | broadly covered; `get_projects_by_owner` → `transfer`, `invariants`, _archive_ |
| `get_projects_by_ids` | **none — GAP** |
| `set_project_lifecycle_status`, `list_projects_by_lifecycle` | `lifecycle_status` |
| `list_projects_by_status`, `list_projects_by_category`, `list_projects_sorted` | `archival`, `sorted_listing`, `issues_242_252_256`, _archive_ |
| `set_project_region`, `get_project_region`, `get_project_integrity_hash` | `region_and_integrity` |
| `archive_project`, `reactivate_project` | `archival`, `lifecycle_status`, _archive_, _emergency_pause_ |
| `initiate_transfer`, `accept_transfer`, `cancel_transfer` | `transfer`, `events`, `archival` |
| `add_maintainer`, `remove_maintainer`, `get_maintainers` | `maintainers`, _auth_matrix_, _reviewer_control_and_tag_index_ |
| `link_project`, `unlink_project`, `get_linked_projects` | `linked_projects`, `duplicate_dispute` |
| `update_security_contact`, `submit_security_contact_proof`, `get_security_contact_status` | `security_contact` |
| `set_featured`, `list_featured_projects` | `featured` |
| `open_duplicate_dispute`, `resolve_duplicate_dispute`, `get_duplicate_dispute`, `get_disputes_for_project` | `duplicate_dispute`, `typed_error_regressions` |
| `set_min_project_age`, `get_min_project_age` | `invariants`, _new_features_, _basic_new_features_, `verification_features` |
| `report_project`, `get_project_reports`, `get_project_report_count`, `has_user_reported`, `clear_project_reports` | `report_registry`, `cleanup`, _new_features_ |

### Reviews & ratings

| Entry points | Covering tests |
|--------------|----------------|
| `add_review`, `update_review`, `delete_review`, `submit_review` | `review`, `review_history`, `invariants`, `moderation`, _review_eligibility_, _canonical_cid_tests_, _atomicity_ |
| `respond_to_review`, `get_review_response` | `review`, _auth_matrix_ |
| `get_review`, `get_review_cid`, `get_project_review_cids`, `list_reviews`, `get_project_stats`, `get_weighted_rating` | `review`, `review_history`, `moderation`, `cleanup`, `pagination` |
| `get_reviews_by_ids` | **none — GAP** |
| `list_reviews_sorted`, `get_review_tombstone` | `review_features` |
| `get_review_history`, `get_review_revision_count` | `review_history` |
| `set_reviews_enabled`, `get_reviews_enabled` | `review_settings`, _atomicity_, `events` |
| `report_review`, `hide_review`, `restore_review`, `admin_delete_review` | `moderation`, `events`, `cleanup`, _auth_matrix_ |
| `get_stats_batch` | _indexer_ only — **weak** |

### Verification

| Entry points | Covering tests |
|--------------|----------------|
| `request_verification`, `approve_verification`, `reject_verification`, `revoke_verification` | `verification`, `verification_lifecycle`, `verification_replacement`, `renewal`, `multisig_and_history`, `fee_refund`, _verified_freeze_ |
| `update_verification_evidence` | `verification` |
| `get_verification`, `get_verification_record`, `get_verification_history`, `get_pending_verifications` | `verification`, `verification_replacement`, `verification_lifecycle` |
| `get_verifications_batch`, `get_verification_records_batch` | `get_verifications_batch` → _indexer_ (weak); `get_verification_records_batch` → **none — GAP** |
| `is_verification_active`, `is_verification_expired`, `is_verification_expiring_soon` | `verification_lifecycle`, `verification_features`, `renewal` |
| `renew_verification` | **none — GAP** (only the request/approve renewal flow is tested) |
| `request_renewal`, `approve_renewal`, `reject_renewal`, `get_renewal_request`, `get_renewal_history` | `renewal`, `verification_lifecycle`, `cleanup` |
| `clear_verification_history`, `clear_renewal_history` | `cleanup` |
| `assign_verification`, `get_assigned_admin` | _verification_assignment_, `typed_error_regressions` |
| `set_verification_duration`, `get_verification_duration` | `verification_features`, `verification_lifecycle`, `renewal` |
| `get_fee_refund`, `claim_fee_refund` | `fee_refund` |
| `add_reserved_name`, `remove_reserved_name`, `get_reserved_names`, `is_name_reserved` | _reserved_names_ only — **not compiled** |

### Fees

| Entry points | Covering tests |
|--------------|----------------|
| `set_fee`, `pay_fee`, `is_fee_paid`, `cancel_fee_payment` | `fee_lifecycle`, `fee_refund`, `fee_boundary`, `fee_token_rotation`, `_fee_`, `atomicity`_ |
| `pay_registration_fee`, `get_reg_fee_payment_details` | `pay_registration_fee` → `fee_boundary`; `get_reg_fee_payment_details` → **none — GAP** |
| `get_fee_config`, `get_fee_config_history`, `get_config` | `fee_token_rotation`, `config`, `typed_error_regressions` |
| `get_fee_payment_details` | _fee_payment_details_, `fee_lifecycle` |

### Discovery, social, collections, changelog, claims, dependencies

| Entry points | Covering tests |
|--------------|----------------|
| `create_collection`, `update_collection`, `delete_collection`, `add_project_to_collection`, `remove_project_from_collection`, `get_collection`, `list_collections`, `list_collection_projects`, `get_collection_project_count`, `get_collection_count` | `collection_registry_crud`, `collections`, _auth_matrix_ |
| `follow_project`, `unfollow_project`, `get_follower_count`, `is_following`, `get_project_followers`, `get_user_subscriptions` | `subscriptions`, _emergency_pause_ |
| `bookmark_project`, `unbookmark_project`, `is_bookmarked`, `get_user_bookmarks` | `bookmarks`, `bookmark_pagination`, _emergency_pause_ |
| `endorse_project`, `unendorse_project`, `get_endorsement_count`, `has_endorsed` | `endorsements`, _review_eligibility_, _emergency_pause_ |
| `get_projects_by_tag_batch`, `list_projects_by_tag`, `reindex_tags`, `get_tag_index_watermark` | `tag_index`, _reviewer_control_and_tag_index_, `archival`, _new_features_ |
| `add_changelog_entry`, `remove_changelog_entry`, `get_changelog_entry`, `get_project_changelog`, `get_changelog_count` | `changelog` |
| `set_project_claimable`, `submit_claim_request`, `approve_claim_request`, `reject_claim_request`, `get_claim_request`, `get_claim_requests_for_project` | `claim`, _claim_status_, `events`, _auth_matrix_ |
| `claim_contract_address`, `approve_contract_claim`, `reject_contract_claim`, `get_verified_contracts` | _claim_status_, `issues_242_252_256` |
| `add_project_dependency`, `update_project_dependency`, `remove_project_dependency`, `get_project_dependencies`, `get_project_dependency_count` | `dependencies`, _auth_matrix_; `get_project_dependency_count` → **none — GAP** |

### TTL / storage maintenance

| Entry points | Covering tests |
|--------------|----------------|
| `extend_projects_ttl`, `extend_reviews_ttl` | `ttl_batch` |
| `extend_project_ttl`, `extend_review_ttl`, `extend_admin_ttl`, `extend_user_ttl`, `extend_verification_ttl`, `extend_critical_config_ttl` | **none — GAP** (only the batch variants are tested) |

### Indexer / batch APIs

| Entry points | Covering tests |
|--------------|----------------|
| `get_projects_by_ids`, `get_reviews_by_ids`, `get_verification_records_batch` | **none — GAP** |
| `get_stats_batch`, `get_verifications_batch` | _indexer_ only — **not compiled** |
| `api_compat` surface | `api_compat` |

---

## 5. Known gaps

### 5.1 Uncovered entry points (no test references any test file)

```
set_pause
renew_verification
get_projects_by_ids
get_reviews_by_ids
get_verification_records_batch
get_reg_fee_payment_details
get_project_dependency_count
get_admin_action_log_by_admin
extend_project_ttl
extend_review_ttl
extend_admin_ttl
extend_user_ttl
extend_verification_ttl
extend_critical_config_ttl
```

Most are getters or single-item TTL extenders whose batch equivalents are
tested. `set_pause` and `renew_verification` are behavioural and should get
dedicated tests.

### 5.2 Coverage that exists only in non-compiled files

Re-enabling these (per `TEST_ORGANIZATION.md`) recovers real coverage:

| Feature | Only-source file | Tests | Impact |
|---------|------------------|------:|--------|
| Emergency pause | _emergency_pause.rs_ | 23 | **Critical path** — pause currently has no active coverage |
| Verified-field freeze | _verified_freeze.rs_ | 19 | **Critical path** — backs THREAT_MODEL §3.A |
| Authorization matrix | _auth_matrix.rs_ (#215) | 35 | role × function negative tests |
| Reserved names | _reserved_names.rs_ | 7 | naming policy |
| Review eligibility (anti-sybil) | _review_eligibility.rs_ | 17 | THREAT_MODEL §3.B |
| Atomicity | _atomicity.rs_ | 29 | multi-storage invariants |
| Verification assignment | _verification_assignment.rs_ (#227) | 5 | admin assignment |
| Indexer batch APIs | _indexer.rs_ | 13 | `get_stats_batch`, `get_verifications_batch` |
| Storage index limits | _index_limits.rs_ | 5 | owner/review index caps |
| Canonical CID | _canonical_cid_tests.rs_ | 11 | no data loss on consolidation |

### 5.3 Measured coverage

| Date | Tool | Overall % | Critical-path % | Report |
|------|------|----------:|----------------:|--------|
| _pending_ | `cargo tarpaulin --lib` | _run §1_ | _run §1_ | `coverage/tarpaulin-report.html` |

Fill this row in on each release. The first measured run establishes the
baseline against the 85 % target.

---

## 6. Roadmap to the 85 % gate

1. **Re-enable the non-compiled critical-path suites** (`emergency_pause.rs`,
   `verified_freeze.rs`, `auth_matrix.rs`) — `TEST_ORGANIZATION.md` §2.
2. **Add tests for the §5.1 behavioural gaps** (`set_pause`,
   `renew_verification`, single-item TTL extenders, `*_by_ids` batch getters).
3. **Run §1 and record the baseline** in §5.3.
4. **Add a non-blocking `coverage` job to `ci.yml`** that runs
   `cargo tarpaulin --lib --out Xml`, uploads the report as an artifact, and
   comments the delta on the PR. Keep it non-blocking until the baseline clears
   85 %.
5. **Flip the job to blocking** with `--fail-under 85` (overall) once step 3
   confirms headroom; enforce 100 % on the §3 critical-path files via a
   scoped second tarpaulin invocation.

---

**Last Updated:** 2026-08-27
**Scope:** `dongle-contract` v0.6.0
**Related docs:** [`TEST_ORGANIZATION.md`](TEST_ORGANIZATION.md), [`CI_CD.md`](CI_CD.md), [`TESTING.md`](TESTING.md), [`THREAT_MODEL.md`](THREAT_MODEL.md), [`CONTRACT_INTERFACE.md`](CONTRACT_INTERFACE.md)
