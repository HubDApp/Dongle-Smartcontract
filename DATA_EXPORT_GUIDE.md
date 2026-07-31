# Data Export Guide for Indexers

This guide explains how to reconstruct and maintain full Dongle contract state from on-chain reads and contract events. It covers initial backfill, event-driven incremental sync, pagination patterns, and recovery from missed events.

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Initial Backfill](#initial-backfill)
4. [Event-Driven Incremental Sync](#event-driven-incremental-sync)
5. [Pagination Examples](#pagination-examples)
6. [Recovery Strategy for Missed Events](#recovery-strategy-for-missed-events)
7. [Event Reference](#event-reference)

---

## Overview

The Dongle smart contract stores project registry data on the Stellar network using Soroban persistent storage. Indexers are responsible for:

- **Backfilling** the full project list at startup.
- **Listening to contract events** to apply incremental state changes.
- **Handling gaps** caused by missed or out-of-order events.

The combination of read-based backfill and event-driven updates ensures your indexed state stays consistent with the on-chain truth.

---

## Prerequisites

- **Stellar RPC endpoint** – e.g. `https://soroban-testnet.stellar.org:443` for testnet.
- **Contract address** – from [deployments.json](./deployments.json).
- **Soroban SDK or Stellar JS SDK** – to invoke read functions and subscribe to events.
- **Local database** – to persist indexed state between restarts (e.g. PostgreSQL, SQLite).
- **Checkpoint store** – to persist the last processed ledger sequence so restarts resume correctly.

---

## Initial Backfill

Backfill is a one-time (or on-restart) operation that fetches all current on-chain state before the indexer begins consuming events.

### Step 1 – Record the starting ledger

Before querying any data, note the **current ledger sequence**. Events emitted at or after this ledger will be consumed during incremental sync. This avoids a race condition between backfill and live events.

```js
const startLedger = await rpc.getLatestLedger(); // save this value
```

### Step 2 – Paginate through all projects

Use `list_projects(start, limit)` to page through every registered project. Continue until the returned list is shorter than `limit`, which signals the last page.

```js
const PAGE_SIZE = 50;
let start = 0;

while (true) {
  const projects = await contract.list_projects({ start, limit: PAGE_SIZE });

  for (const project of projects) {
    await db.upsert("projects", project); // store or update local record
  }

  if (projects.length < PAGE_SIZE) break; // last page reached
  start += PAGE_SIZE;
}
```

### Step 3 – Fetch supporting data

After loading projects, backfill associated data for each project:

```js
for (const project of allProjects) {
  const reviews    = await contract.list_reviews({ project_id: project.id, start: 0, limit: 50 });
  const stats      = await contract.get_project_stats({ project_id: project.id });
  const linked     = await contract.get_linked_projects({ project_id: project.id });
  const disputes   = await contract.get_disputes_for_project({ project_id: project.id });

  await db.upsert("reviews",  reviews);
  await db.upsert("stats",    stats);
  await db.upsert("links",    linked);
  await db.upsert("disputes", disputes);
}
```

### Step 4 – Save the checkpoint

After backfill completes, persist `startLedger` as your sync cursor so the event stream can resume from exactly this point.

```js
await db.set("last_synced_ledger", startLedger);
```

---

## Event-Driven Incremental Sync

After backfill, the indexer processes contract events to apply fine-grained state changes without re-reading all data.

### Polling loop

```js
const POLL_INTERVAL_MS = 5000;
const BATCH_SIZE = 200;

async function syncLoop() {
  while (true) {
    const fromLedger = await db.get("last_synced_ledger");

    const { events, latestLedger } = await rpc.getEvents({
      startLedger: fromLedger,
      filters: [{ contractIds: [CONTRACT_ADDRESS] }],
      limit: BATCH_SIZE,
    });

    for (const event of events) {
      await handleEvent(event);
    }

    await db.set("last_synced_ledger", latestLedger);
    await sleep(POLL_INTERVAL_MS);
  }
}
```

### Event handler

The current contract publishes Soroban topics as tuples of symbols rather than a single event-name string. The live implementation uses topic patterns such as `("PROJECT", "CREATED", project_id)`, `("VERIFY", "REQ", project_id)`, `("REVIEW", "SUBMITTED", project_id, reviewer)`, and `("FEE", "PAID", project_id)`.

```js
async function handleEvent(event) {
  const [namespace, action, projectId] = event.topic;

  switch (namespace) {
    case "PROJECT":
      switch (action) {
        case "CREATED":
          await db.upsert("projects", parseProjectRegistered(event));
          break;

        case "UPDATED": {
          const { project_id } = parseProjectUpdated(event);
          const fresh = await contract.get_project({ project_id });
          await db.upsert("projects", fresh);
          break;
        }

        case "ARCHIVED":
          await db.update("projects", { id: event.data.project_id, status: "archived" });
          break;

        case "RESTORED":
          await db.update("projects", { id: event.data.project_id, status: "active" });
          break;

        case "TRANSFER":
          await db.update("projects", {
            id: event.data.project_id,
            owner: event.data.new_owner,
          });
          break;

        default:
          break;
      }
      break;

    case "VERIFY":
      switch (action) {
        case "REQ":
        case "APP":
        case "REJ":
        case "REVOKED":
          await handleVerificationEvent(event);
          break;
        default:
          break;
      }
      break;

    case "REVIEW":
      await handleReviewEvent(event);
      break;

    case "DISPUTE":
      await handleDisputeEvent(event);
      break;

    default:
      console.warn("Unhandled event topic:", event.topic, event);
  }
}
```

### Verification event handler example

```js
async function handleVerificationEvent(event) {
  const { project_id } = event.data;

  // Re-read the project to get the latest verified status
  const project = await contract.get_project({ project_id });
  await db.upsert("projects", project);
}
```

---

## Pagination Examples

All list functions share the same `(start, limit)` pattern. Use these helpers for consistent pagination.

### Generic page iterator

```js
async function* paginate(fn, pageSize = 50) {
  let start = 0;
  while (true) {
    const page = await fn(start, pageSize);
    yield* page;
    if (page.length < pageSize) return;
    start += pageSize;
  }
}
```

### Paginate projects

```js
for await (const project of paginate((s, l) => contract.list_projects({ start: s, limit: l }))) {
  console.log(project.id, project.name);
}
```

### Paginate reviews for a project

```js
for await (const review of paginate((s, l) => contract.list_reviews({ project_id: 42, start: s, limit: l }))) {
  console.log(review.reviewer, review.rating);
}
```

### Paginate featured projects

```js
for await (const featured of paginate((s, l) => contract.list_featured_projects({ start: s, limit: l }))) {
  console.log(featured);
}
```

### Paginate collections

```js
for await (const collection of paginate((s, l) => contract.list_collections({ start: s, limit: l }))) {
  console.log(collection.id, collection.name);
}
```

### Batch stats fetch

For efficiency, use `get_stats_batch` when you need stats for many projects at once:

```js
const projectIds = [1, 2, 3, 4, 5];
const statsBatch = await contract.get_stats_batch({ project_ids: projectIds });

for (const [id, stats] of Object.entries(statsBatch)) {
  console.log(id, stats.average_rating, stats.review_count);
}
```

---

## Recovery Strategy for Missed Events

Events may be missed due to RPC downtime, network errors, or indexer restarts. The following strategy ensures full consistency.

### How gaps occur

- The indexer is offline for several ledgers.
- An RPC error causes a batch of events to be dropped silently.
- The checkpoint was saved before events were fully processed.

### Detect a gap

Compare your stored `last_synced_ledger` with the current ledger. If the difference exceeds your expected polling window, treat it as a potential gap:

```js
const lastSynced = await db.get("last_synced_ledger");
const current    = await rpc.getLatestLedger();

const GAP_THRESHOLD = 500; // ledgers (~50 minutes on Stellar)
if (current - lastSynced > GAP_THRESHOLD) {
  console.warn("Gap detected. Running reconciliation.");
  await reconcile(lastSynced);
}
```

### Reconcile with a targeted re-read

When a gap is detected, fetch all events between the last known ledger and the current one, then re-apply them in order:

```js
async function reconcile(fromLedger) {
  let cursor = fromLedger;

  while (true) {
    const { events, latestLedger } = await rpc.getEvents({
      startLedger: cursor,
      filters: [{ contractIds: [CONTRACT_ADDRESS] }],
      limit: 200,
    });

    for (const event of events) {
      await handleEvent(event);
    }

    if (events.length < 200) {
      await db.set("last_synced_ledger", latestLedger);
      break;
    }

    cursor = events[events.length - 1].ledger + 1;
  }
}
```

### Full re-sync fallback

If the gap is too large for event replay (events pruned from the RPC node), fall back to a full backfill:

```js
async function fullResync() {
  await db.clear("projects");
  await db.clear("reviews");
  await db.clear("stats");
  await db.clear("disputes");

  await initialBackfill(); // re-runs all steps from Initial Backfill section
}
```

### Idempotent writes

All database writes should use **upsert** (insert-or-update) semantics. This ensures that replaying an event or re-reading a project during reconciliation is safe and does not produce duplicates or corrupt state.

```js
// Safe: re-applying the same event produces the same result
await db.upsert("projects", { id: project.id, ...fields });
```

### Checkpoint commit order

Always update the checkpoint **after** all events in a batch are processed and committed to your database — not before. This guarantees that a crash mid-batch causes a safe re-replay rather than a silent data loss:

```js
for (const event of events) {
  await handleEvent(event);       // process first
}
await db.set("last_synced_ledger", latestLedger); // commit checkpoint last
```

---

## Event Reference

The following event shapes are emitted by the current contract implementation. Topics are published as Soroban topic tuples, and the payload is available on `event.data`. The storage-key enum in [dongle-smartcontract/src/storage_keys.rs](./dongle-smartcontract/src/storage_keys.rs) is an internal implementation detail for persistence and should not be treated as an external event schema.

| Event | Topic tuple | Key fields in `event.data` |
|---|---|---|
| Project registered | `("PROJECT", "CREATED", project_id)` | `project_id`, `owner`, `name`, `category`, `timestamp` |
| Project updated | `("PROJECT", "UPDATED", project_id)` | `project_id`, `owner`, `timestamp` |
| Project archived | `("PROJECT", "ARCHIVED", project_id)` | `project_id`, `archived_by`, `timestamp` |
| Project reactivated | `("PROJECT", "RESTORED", project_id)` | `project_id`, `caller`, `timestamp` |
| Ownership transferred | `("PROJECT", "TRANSFER", project_id)` | `project_id`, `caller`, `old_owner`, `new_owner`, `timestamp` |
| Verification requested | `("VERIFY", "REQ", project_id)` | `project_id`, `requester`, `evidence_cid`, `timestamp` |
| Verification approved | `("VERIFY", "APP", project_id)` | `project_id`, `admin`, `decided_at`, `timestamp` |
| Verification rejected | `("VERIFY", "REJ", project_id)` | `project_id`, `admin`, `decided_at`, `timestamp` |
| Verification revoked | `("VERIFY", "REVOKED", project_id)` | `project_id`, `admin`, `reason`, `timestamp` |
| Verification evidence updated | `("VERIFY", "EV_UPD", project_id)` | `project_id`, `requester`, `old_evidence_cid`, `new_evidence_cid`, `timestamp` |
| Verification renewal requested | `("RENEW", "REQUEST", project_id)` | `project_id`, `requester`, `evidence_cid`, `fee_amount`, `timestamp` |
| Verification renewal approved | `("RENEW", "APPROVED", project_id)` | `project_id`, `admin`, `expires_at`, `timestamp` |
| Verification renewal rejected | `("RENEW", "REJECTED", project_id)` | `project_id`, `admin`, `timestamp` |
| Review action | `("REVIEW", "SUBMITTED" \| "UPDATED" \| "REVISED" \| "DELETED", project_id, reviewer)` | `project_id`, `reviewer`, `action`, `timestamp`, `content_cid`, `created_at`, `updated_at`, `owner_response` |
| Review reported | `("REVIEW", "REPORTED", project_id)` | `project_id`, `reviewer`, `reporter`, `timestamp` |
| Review hidden | `("REVIEW", "HIDDEN", project_id)` | `project_id`, `reviewer`, `admin`, `timestamp` |
| Review restored | `("REVIEW", "RESTORED", project_id)` | `project_id`, `reviewer`, `admin`, `timestamp` |
| Review deleted by admin | `("REVIEW", "ADMINDEL", project_id)` | `project_id`, `reviewer`, `admin`, `timestamp` |
| Dispute opened | `("DISPUTE", "OPENED", project_id)` | `project_id`, `reporter`, `timestamp` |
| Dispute resolved | `("DISPUTE", "RESOLVED", project_id)` | `project_id`, `admin`, `resolution`, `timestamp` |
| Project reported | `("PROJECT", "REPORTED", project_id)` | `project_id`, `reporter`, `reason_cid`, `timestamp` |
| Fee paid | `("FEE", "PAID", project_id)` | `project_id`, `payer`, `token`, `operation`, `amount`, `timestamp` |
| Fee consumed | `("FEE", "CONSUMED", project_id)` | `project_id`, `caller`, `operation`, `amount`, `timestamp` |
| Admin added | `("ADMIN", "ADDED")` | `admin`, `timestamp` |
| Admin removed | `("ADMIN", "REMOVED")` | `admin`, `timestamp` |

For the complete list of contract functions used in backfill, see [CONTRACT_INTERFACE.md](./CONTRACT_INTERFACE.md).
