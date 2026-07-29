# PR: Add Data Export Guide for Indexers

## Summary

This pull request adds `DATA_EXPORT_GUIDE.md`, a comprehensive reference for anyone building an indexer on top of the Dongle smart contract. The guide explains how to reconstruct and maintain full contract state from on-chain reads and contract events, covering all four acceptance criteria from issue #261.

---

## What Was Added

### `DATA_EXPORT_GUIDE.md`

A single, self-contained guide at the repository root. It is organised into the following sections:

---

### 1. Initial Backfill

Documents the four-step process for building a complete local snapshot of contract state before the indexer begins consuming live events:

1. **Record the starting ledger** – capture the current ledger sequence before any reads so incremental sync can start from exactly the right point and avoid a race condition between backfill and live events.
2. **Paginate through all projects** – use `list_projects(start, limit)` in a loop, advancing `start` by `PAGE_SIZE` until the returned slice is shorter than the page size, signalling end-of-data.
3. **Fetch supporting data** – for each project, backfill reviews (`list_reviews`), stats (`get_project_stats`), linked projects (`get_linked_projects`), and disputes (`get_disputes_for_project`).
4. **Save the checkpoint** – persist `startLedger` to the local store so restarts resume from the correct ledger.

---

### 2. Event-Driven Incremental Sync

Documents how to consume contract events after backfill to apply fine-grained state changes without re-reading all data:

- **Polling loop** – calls `rpc.getEvents` on a configurable interval (`POLL_INTERVAL_MS`), filtering by contract address, and advances the checkpoint after each batch.
- **Event handler** – a `switch` block routes each event topic to the correct update logic. For lightweight events (e.g. `ProjectArchived`) the handler patches a single field. For richer events (e.g. `ProjectUpdated`, any verification event) it re-fetches the full record via `get_project` to guarantee consistency.
- **Full coverage** – handlers are shown for: project lifecycle (`Registered`, `Updated`, `Archived`, `Reactivated`, `OwnershipTransferred`), verification lifecycle (`Requested`, `Approved`, `Rejected`, `Revoked`), reviews (`Submitted`, `Updated`, `Deleted`), and disputes (`Opened`, `Resolved`).

---

### 3. Pagination Examples

Documents the `(start, limit)` pagination pattern shared by all list functions, with reusable helpers:

- **Generic async page iterator** (`paginate`) – a JavaScript async generator that wraps any list function and yields items page by page.
- **Per-entity examples** – concrete usages of the iterator for projects, reviews, featured projects, and collections.
- **Batch stats fetch** – demonstrates `get_stats_batch` for efficient multi-project stats retrieval in a single call.

---

### 4. Recovery Strategy for Missed Events

Documents a layered approach to handle gaps caused by RPC downtime, network errors, or indexer restarts:

- **Gap detection** – compares `last_synced_ledger` against the current ledger and triggers reconciliation when the difference exceeds a configurable `GAP_THRESHOLD`.
- **Targeted reconciliation** – replays all events between the last checkpoint and the current ledger, processing them in order.
- **Full re-sync fallback** – clears local state and re-runs the full backfill when the gap is too large for event replay (events pruned from the RPC node).
- **Idempotent writes** – all writes use upsert semantics so replaying an event or re-reading a record is always safe.
- **Checkpoint commit order** – explicitly documents that the checkpoint must be updated *after* all events in a batch are committed to avoid silent data loss on crash.

---

### 5. Event Reference Table

A complete table of every contract event, its topic symbol, and its key fields, covering:

| Category | Events |
|---|---|
| Projects | `ProjectRegistered`, `ProjectUpdated`, `ProjectArchived`, `ProjectReactivated`, `OwnershipTransferred` |
| Verification | `VerificationRequested/Approved/Rejected/Revoked`, `VerificationRenewalRequested/Approved/Rejected` |
| Reviews | `REVIEW` (Submitted/Updated/Deleted), `ReviewReported`, `ReviewHidden`, `ReviewRestored` |
| Disputes | `DisputeOpened`, `DisputeResolved` |
| Moderation | `ProjectReported` |
| Fees | `FeePaid` |
| Admin | `AdminAdded`, `AdminRemoved` |

---

## Testing

This PR adds documentation only — no contract code was modified. The existing test suite passes unchanged.

---

## Checklist

- [x] `DATA_EXPORT_GUIDE.md` created at repository root
- [x] Initial backfill documented (step-by-step with code examples)
- [x] Event-driven incremental sync documented (polling loop + full event handler)
- [x] Pagination examples included (generic iterator + per-entity + batch stats)
- [x] Recovery strategy documented (gap detection, reconciliation, full re-sync, idempotency, checkpoint ordering)
- [x] Full event reference table included
- [x] All code examples use JavaScript (consistent with existing repo examples)
- [x] Cross-references to `CONTRACT_INTERFACE.md` and `deployments.json` added

---

closes #261
