# Review Moderation

The review moderation flow lets authenticated users report reviews and lets
contract administrators hide or restore them. Hidden reviews remain available
for direct inspection, but are excluded from normal listings and project rating
statistics.

This guide reflects the implementation under `dongle-smartcontract/src`.

## Public API

| Method | Caller | Result |
|--------|--------|--------|
| `report_review(project_id, reviewer, reporter)` | Authenticated reporter | Records one report per reporter and increments the review's report count |
| `hide_review(project_id, reviewer, admin)` | Contract administrator | Hides the review and removes its rating from project statistics |
| `restore_review(project_id, reviewer, admin)` | Contract administrator | Restores the review and adds its rating back to project statistics |
| `get_review(project_id, reviewer)` | Public | Returns the review even when it is hidden |
| `list_reviews(project_id, offset, limit)` | Public | Returns visible reviews only |

The public contract entrypoints are defined in
[`lib.rs`](../dongle-smartcontract/src/lib.rs), and their behavior is
implemented by
[`ReviewRegistry`](../dongle-smartcontract/src/review_registry.rs).

## Reporting a Review

`report_review` requires the `reporter` address to authorize the call. It then:

1. verifies that the project and review exist;
2. checks `ReviewReport(project_id, reviewer, reporter)` to prevent a duplicate
   report from the same address;
3. increments `Review.report_count` with saturating arithmetic;
4. stores the per-reporter deduplication key;
5. extends the review's storage lifetime; and
6. emits a `ReviewReportedEvent`.

Different users can report the same review. A single user cannot report the
same review more than once.

```rust
client.report_review(&project_id, &reviewer, &reporter);
let review = client.get_review(&project_id, &reviewer).unwrap();
assert_eq!(review.report_count, 1);
```

## Hiding a Review

`hide_review` requires authorization from an address registered as a contract
administrator. After validating the project and review, it:

1. rejects a review that is already hidden;
2. sets `Review.hidden` to `true`;
3. removes the review rating from `ProjectStats` when the project has at least
   one counted review;
4. extends the review and project-statistics storage lifetimes;
5. emits a `ReviewHiddenEvent`; and
6. records `AdminActionType::ReviewHidden` in the administrator action log.

```rust
client.hide_review(&project_id, &reviewer, &admin);
assert!(client.get_review(&project_id, &reviewer).unwrap().hidden);
```

## Restoring a Review

`restore_review` has the same administrator checks. It:

1. rejects a review that is not hidden;
2. sets `Review.hidden` to `false`;
3. adds the review rating back to `ProjectStats`;
4. extends the review and project-statistics storage lifetimes;
5. emits a `ReviewRestoredEvent`; and
6. records `AdminActionType::ReviewRestored` in the administrator action log.

```rust
client.restore_review(&project_id, &reviewer, &admin);
assert!(!client.get_review(&project_id, &reviewer).unwrap().hidden);
```

Repeated hide and restore cycles update the statistics once per state
transition. The report count is preserved across both operations.

## Review Data

Moderation uses these fields on the current `Review` type:

```rust
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,
    pub content_cid: Option<String>,
    pub owner_response: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub hidden: bool,
    pub report_count: u32,
}
```

`content_cid` is the canonical content reference. Older documentation referred
to separate `ipfs_cid` and `comment_cid` fields, which are no longer part of the
type.

## Storage and Events

The persistent deduplication key is:

```rust
StorageKey::ReviewReport(project_id, reviewer, reporter)
```

The three moderation events share `project_id`, `reviewer`, and a ledger
timestamp. Report events include the reporter; hide and restore events include
the administrator.

| Event | Topic suffix | Actor field |
|-------|--------------|-------------|
| `ReviewReportedEvent` | `REVIEW`, `REPORTED`, `project_id` | `reporter` |
| `ReviewHiddenEvent` | `REVIEW`, `HIDDEN`, `project_id` | `admin` |
| `ReviewRestoredEvent` | `REVIEW`, `RESTORED`, `project_id` | `admin` |

Event definitions and publishers live in
[`events.rs`](../dongle-smartcontract/src/events.rs). The deduplication key is
defined in
[`storage_keys.rs`](../dongle-smartcontract/src/storage_keys.rs).

## Visibility and Rating Statistics

- `get_review` can retrieve a hidden review for moderation and audit workflows.
- `list_reviews` and sorted review listings skip hidden reviews.
- Hiding removes the review's rating and decrements the counted-review total.
- Restoring adds the rating and increments the counted-review total.
- The average rating is recalculated through `RatingCalculator` after each
  transition.

Clients that display review totals should use `ProjectStats` rather than count
the result of a single paginated listing.

## Expected Errors

The moderation implementation can return project, review, authorization, and
state errors, including:

- `ProjectNotFound`
- `ReviewNotFound`
- `AdminOnly`
- `ReviewAlreadyReported`
- `ReviewAlreadyHidden`
- `ReviewNotHidden`

### Current source consistency issue

At the time this guide was consolidated, `review_registry.rs` referenced the
three moderation-specific variants above, but `errors.rs` did not declare them.
Numeric error codes from the old guides were therefore removed because they
conflicted with the current enum. The variants must be added to a non-conflicting
range before the contract can compile with these paths, and the chosen codes
should then be documented in the canonical error reference.

## Test Coverage

The focused suite in
[`tests/moderation.rs`](../dongle-smartcontract/src/tests/moderation.rs)
contains 24 tests covering:

- successful reporting and multiple independent reporters;
- duplicate-report and missing project or review failures;
- administrator authorization for hide and restore operations;
- statistics updates during hide and restore transitions;
- visible-review listing behavior;
- repeated hide and restore cycles;
- report-count preservation; and
- isolation between projects.

Moderation behavior is also exercised by the authorization matrix, event, and
cleanup suites. Run the contract tests from the workspace root with:

```bash
cargo test -p dongle-contract
```

## Operational Notes

- Reporting does not automatically hide a review.
- There is no report-reason field or on-chain moderation queue in this flow.
- Administrators should use emitted events and the action log for off-chain
  audit trails.
- Indexers should treat hide and restore events as visibility transitions, not
  review deletion or creation.
