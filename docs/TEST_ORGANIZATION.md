# Test Suite Organization

The contract test suite lives in `dongle-smartcontract/src/tests/` and is
compiled as a single `#[cfg(test)] mod tests;` tree (see
`dongle-smartcontract/src/lib.rs`). Every test file must be declared in
`src/tests/mod.rs` to be compiled and run — a `.rs` file that is not declared
there is **dead code that never runs**.

This document is the map of that suite: what exists, what actually runs, the
naming convention, where the duplication is, and the target structure.

---

## 1. Current inventory (2026-08-27)

| Metric | Count |
|--------|------:|
| `.rs` files in `src/tests/` | 81 |
| — `fixtures.rs` / `mod.rs` (infra, no tests) | 2 |
| — `error_handling_tests.rs` (empty stub, 1 byte) | 1 |
| Test files with `#[test]` functions | 78 |
| `#[test]` functions on disk (all files) | ~1011 |
| Modules **declared and active** in `mod.rs` | 55 |
| Modules **commented out** in `mod.rs` (disabled) | 9 |
| Files **never referenced** in `mod.rs` (orphaned) | 17 |

**~26 test files — roughly 370 `#[test]` functions — are on disk but not
compiled.** That is the core of the maintenance burden in issue #614: the file
count implies far more coverage than the suite actually exercises, and the
orphaned/disabled files rot against API changes because nothing builds them.

Regenerate these numbers:

```bash
cd dongle-smartcontract
ls src/tests/*.rs | wc -l                                   # files
grep -rho '#\[test\]' src/tests/ | wc -l                    # test fns on disk
grep -cE '^(pub )?mod [a-z_0-9]+;' src/tests/mod.rs         # active
grep -cE '^// mod ' src/tests/mod.rs                        # disabled
```

---

## 2. The three states

### 2.1 Active (55) — compiled and run by `cargo test`

```
admin              admin_action_log   api_compat         archival
bookmark_pagination bookmarks         changelog          claim
cleanup            collection_registry_crud  collections config
dependencies       duplicate_dispute  endorsements       error_handling_tests
events             featured           fee_boundary       fee_lifecycle
fee_refund         fee_token_rotation field_limits       fixtures
invariants         issues_242_252_256 license_metadata   lifecycle_status
linked_projects    maintainers        moderation         multisig_and_history
proposal_threshold proptest_pagination region_and_integrity  renewal
report_registry    review             review_features    review_history
review_settings    security_contact   sorted_listing     string_validation
subscriptions      tag_index          tags               timelock
transfer           ttl_batch          typed_error_regressions  verification
verification_features  verification_lifecycle  verification_replacement
```

> Note: `mod.rs` currently declares `proptest_pagination` twice — once
> commented (`// mod proptest_pagination;`) and once active. The commented line
> is redundant and should be deleted.

### 2.2 Disabled (9) — commented out in `mod.rs`

| File | `#[test]` | Header | Disposition |
|------|----------:|--------|-------------|
| `atomicity.rs` | 29 | Atomicity guarantees for multi-storage ops | **Re-enable** — high-value invariant tests, or fold into `invariants.rs` |
| `authorization.rs` | 23 | Unauthorized-access tests for every mutating endpoint | **Re-enable** — overlaps `auth_matrix.rs` (§4); pick one |
| `basic_new_features.rs` | 7 | "verify they compile and work" smoke tests | **Delete** — superseded by the domain files for those features |
| `fee.rs` | 13 | Owner-bound verification fee payments | **Merge** into `fee_lifecycle.rs` / `fee_refund.rs` |
| `index_limits.rs` | 5 | Storage index size limits (owner projects, reviews) | **Merge** into `invariants.rs` |
| `indexer.rs` | 13 | Stable batch APIs used by indexers | **Merge** into `api_compat.rs` |
| `pagination.rs` | 21 | Pagination behaviour | **Merge** into `proptest_pagination.rs` → `pagination.rs` (§5) |
| `proptest_pagination.rs` | 4 | (redundant commented line only) | Delete the commented line |
| `verified_freeze.rs` | 19 | Verified-project metadata freeze | **Re-enable** — directly backs THREAT_MODEL §3.A "Verified Field Freezing" |

### 2.3 Orphaned (17) — on disk, never declared in `mod.rs`

| File | `#[test]` | Header / issue | Disposition |
|------|----------:|----------------|-------------|
| `archive.rs` | 19 | Project archive/reactivate | **Merge** with active `archival.rs` — near-duplicate names, same feature |
| `auth_matrix.rs` | 35 | Role × Function authorization matrix (#215) | **Re-enable as the canonical auth suite**; retire `authorization.rs` |
| `bounty_metadata.rs` | 8 | Bounty metadata | **Consolidate** the 3 bounty files → `bounty.rs` |
| `bounty_test.rs` | 4 | Bounty | ” |
| `test_bounty.rs` | 4 | Bounty | ” |
| `canonical_cid_tests.rs` | 11 | Canonical CID consolidation, no data loss | **Re-enable** → rename `canonical_cid.rs` |
| `claim_status.rs` | 9 | Canonical `ClaimStatus` enum regressions | **Merge** into active `claim.rs` |
| `emergency_pause.rs` | 23 | Contract pause / emergency stop | **Re-enable** — pause has **no active coverage** at all right now |
| `fee_payment_details.rs` | 3 | Fee payment details getter (#223) | **Merge** into `fee_lifecycle.rs` |
| `launch_timestamp.rs` | 5 | Project launch timestamp (#156) | **Merge** into `lifecycle_status.rs` |
| `new_features.rs` | 8 | min project age, reporting, tags, social links | **Split & merge** into `tags.rs`, `report_registry.rs`, etc.; delete shell |
| `registration.rs` | 3 | Registration | **Merge** into `slug.rs` → `project_registration.rs` (§5) |
| `reserved_names.rs` | 7 | Reserved project names (#231) | **Re-enable** → keep as `reserved_names.rs` |
| `review_eligibility.rs` | 17 | Anti-sybil review eligibility | **Re-enable** — merge into `review.rs` domain group |
| `reviewer_control_and_tag_index.rs` | 17 | Reviewer control (#478) + inverted tag index (#483) | **Split**: reviewer bits → `review.rs`; tag-index bits → `tag_index.rs` |
| `slug.rs` | 20 | Project slug | **Re-enable** → `project_registration.rs` group (§5) |
| `verification_assignment.rs` | 5 | Verification assignment to admin (#227) | **Merge** into `verification.rs` domain group |

---

## 3. Naming convention

Adopt one scheme and apply it during consolidation:

1. **One file per contract domain**, named after the domain — not after a
   feature, an issue, or a test phase.
   `verification.rs`, not `verification_features.rs` / `verification_lifecycle.rs`.
2. **No `_test`, `_tests`, `test_`, `_features`, `basic_`, `new_` prefixes or
   suffixes.** The file is already in `src/tests/`; the `#[test]` attribute
   already says it is a test. `bounty.rs`, not `bounty_test.rs` /
   `test_bounty.rs` / `bounty_metadata.rs`.
3. **Issue-scoped regression files are the exception**, and only when the
   regression does not map cleanly to a domain. Name them
   `issue_<n>[_<n>...].rs` (`issues_242_252_256.rs` follows this).
   `typed_error_regressions.rs` is grandfathered.
4. **Sub-area suffix only when a domain file would otherwise exceed ~800 lines
   or ~40 tests.** Then split by *noun*: `fee_refund.rs`, `fee_rotation.rs` —
   never `fee_basic.rs` / `fee_more.rs`.
5. `#[test]` function names: `test_<subject>_<condition>_<expected>`
   (`test_add_review_duplicate_fails`). This is already the dominant style.
6. Shared setup goes in `fixtures.rs`, not copied per file. Every consolidated
   file should `use super::fixtures::*`.

---

## 4. Duplicate / overlapping functionality

| Domain | Files today | Overlap | Target |
|--------|-------------|---------|--------|
| Authorization | `authorization.rs` (disabled, 23), `auth_matrix.rs` (orphan, 35), plus per-domain `*_unauthorized_*` tests | Both aim to prove "every mutating endpoint rejects the wrong caller" | Keep `auth_matrix.rs` (the #215 matrix), delete `authorization.rs`, leave endpoint-local negative tests in place |
| Archive | `archival.rs` (active, 10), `archive.rs` (orphan, 19) | Same feature, names differ by one letter | Merge into `archive.rs` |
| Bounty | `bounty_metadata.rs` (8), `bounty_test.rs` (4), `test_bounty.rs` (4) — all orphan | Three files, one feature | Merge into `bounty.rs` |
| Verification | `verification.rs` (19), `verification_features.rs` (3), `verification_lifecycle.rs` (4), `verification_replacement.rs` (4), `verification_assignment.rs` (orphan, 5) | `_features` / `_lifecycle` are thin slices, not distinct domains | Merge `_features` + `_lifecycle` + `_assignment` into `verification.rs`; keep `verification_replacement.rs` only if `verification.rs` gets too large |
| Review | `review.rs` (29), `review_features.rs` (8), `review_history.rs` (5), `review_settings.rs` (5), `review_eligibility.rs` (orphan, 17) | `_features` = cooldown/tombstone/sort slices | Merge `_features` + `_eligibility` into `review.rs`; keep `review_history.rs` + `review_settings.rs` (distinct nouns) |
| Fee | `fee.rs` (disabled, 13), `fee_lifecycle.rs` (9), `fee_refund.rs` (16), `fee_token_rotation.rs` (14), `fee_boundary.rs` (5), `fee_payment_details.rs` (orphan, 3) | `fee.rs` + `fee_payment_details.rs` are generic | Merge both into `fee_lifecycle.rs`; keep `_refund` / `_rotation` / `_boundary` (distinct nouns) |
| Pagination | `pagination.rs` (disabled, 21), `proptest_pagination.rs` (active, 4), `bookmark_pagination.rs` (11) | Generic vs proptest vs bookmark-specific | Merge `pagination.rs` + `proptest_pagination.rs` → `pagination.rs`; keep `bookmark_pagination.rs` |
| "New features" | `new_features.rs` (orphan, 8), `basic_new_features.rs` (disabled, 7) | Time-stamped grab-bags | Split contents into the real domain files, delete both shells |
| Tag index | `tag_index.rs` (active, 6), `reviewer_control_and_tag_index.rs` (orphan, 17) | Second file mixes two concerns | Move tag-index tests into `tag_index.rs`, reviewer-control tests into `review.rs` |
| CID | `canonical_cid_tests.rs` (orphan, 11), `license_metadata.rs`, `region_and_integrity.rs` | `_tests` suffix | Rename `canonical_cid.rs`, re-enable |

---

## 5. Target structure

After consolidation, `src/tests/` should contain **~45 domain files**, every
one declared in `mod.rs`, grouped as:

```
Admin & governance     admin, admin_action_log, proposal_threshold,
                       multisig_and_history, emergency_pause, timelock
Projects               project_registration (= slug + registration + reserved_names),
                       project_metadata (= license_metadata, region_and_integrity,
                       launch_timestamp, canonical_cid), lifecycle_status,
                       transfer, maintainers, linked_projects, archive
Verification           verification (+ features + lifecycle + assignment),
                       verification_replacement, verified_freeze
Reviews & ratings      review (+ features + eligibility + reviewer control),
                       review_history, review_settings, moderation, report_registry,
                       invariants
Fees                   fee_lifecycle (+ fee + payment_details), fee_refund,
                       fee_token_rotation, fee_boundary
Discovery / social     tags, tag_index, collections, collection_registry_crud,
                       featured, sorted_listing, bookmarks, bookmark_pagination,
                       endorsements, subscriptions, dependencies, changelog,
                       duplicate_dispute, claim (+ claim_status)
Cross-cutting          auth_matrix, authorization-per-endpoint (inline),
                       string_validation, field_limits, pagination, cleanup,
                       atomicity, ttl_batch, events, api_compat (+ indexer),
                       region_and_integrity, index_limits (→ invariants)
Regression             issues_242_252_256, typed_error_regressions,
                       issue_<n>.rs as needed
Infra                  fixtures, mod
```

Delete after merge: `basic_new_features.rs`, `new_features.rs`,
`error_handling_tests.rs` (empty), `bounty_test.rs`, `test_bounty.rs`,
`authorization.rs` (once `auth_matrix` covers it), `verification_features.rs`,
`verification_lifecycle.rs` if folded, `review_features.rs`, `fee.rs`,
`fee_payment_details.rs`, `indexer.rs`, `index_limits.rs`, `pagination.rs`
(name reused by the merge), `archival.rs` (name → `archive.rs`),
`reviewer_control_and_tag_index.rs`, `canonical_cid_tests.rs` (→ `canonical_cid.rs`).

---

## 6. Migration procedure (per domain, one PR each)

Consolidation must not lose a single `#[test]`. For each domain group:

1. `git mv` / copy the source file's test functions into the target domain
   file, resolving name collisions by making the more specific name win.
2. Replace per-file setup helpers with `use super::fixtures::*`.
3. Update `src/tests/mod.rs`: add the target module if new, delete the merged
   module lines (both active and commented).
4. `cargo test -p dongle-contract` — assert the **total test count does not
   drop** (record it before and after in the PR description).
5. `cargo fmt --all` + `cargo clippy ... -D warnings` (see `docs/CI_CD.md`).
6. Update the coverage map in `docs/TEST_COVERAGE.md` and the tables here.

Do the **re-enable** work (§2.2 / §2.3 files that should run) before the
**merge** work, in separate PRs, so any newly-compiled test that fails against
the current API is triaged on its own.

---

## 7. Coverage map

Which contract function is exercised by which test file is tracked in
[`docs/TEST_COVERAGE.md`](TEST_COVERAGE.md). Keep the two documents in sync:
this file owns *file layout*, `TEST_COVERAGE.md` owns *function → test*
mapping and the measured coverage percentage.

---

**Last Updated:** 2026-08-27
**Related docs:** [`TESTING.md`](TESTING.md), [`TEST_COVERAGE.md`](TEST_COVERAGE.md), [`CI_CD.md`](CI_CD.md), [`CONTRIBUTING.md`](CONTRIBUTING.md)
