# Batch Query Patterns (#938)

Use batch endpoints to fetch many records in one contract call instead of
N individual storage reads.

## Endpoints

- `get_projects_batch(project_ids: Vec<u64>) -> Vec<Project>` (max 100, skips missing)
- `get_reviews_batch(ids: Vec<(u64, Address)>) -> Vec<Review>` (max 100, same visibility filter as `get_reviews_by_ids`)
- `get_verifications_batch(ids: Vec<u64>)` (existing, max 100)
- `get_verification_records_batch(request_ids: Vec<u64>)` (existing, max 100)

Legacy aliases `get_projects_by_ids` / `get_reviews_by_ids` remain for compatibility.

## Cost comparison

Individual queries: N round-trips, N× auth/serialization overhead, N storage scans.
Batch queries: 1 round-trip, single pass over the requested keys, missing IDs skipped.

Rough benchmark (local, 50 projects): 50× `get_project` ≈ 50 storage reads +
50 host round-trips; 1× `get_projects_batch(50 ids)` ≈ 50 storage reads +
1 round-trip. Ledger storage reads are similar, but host overhead and
transaction count drop ~Nx, which dominates in real wallets/dApps.

## Pattern

```js
// JS (stellar-sdk): chunk IDs into ≤100 and batch each chunk.
const chunks = chunk(ids, 100);
const all = [];
for (const c of chunks) all.push(...await contract.get_projects_batch(c));
```

See `dongle-smartcontract/src/tests/` batch tests for correctness expectations.
