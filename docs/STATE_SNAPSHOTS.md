# Contract State Snapshots

Issue #933. Admins can export verifiable snapshots of contract state for
regulatory audits and data recovery.

## What a snapshot contains

`export_contract_state_snapshot(admin, scope, start_project_id, limit)` returns
a `ContractStateSnapshot`:

| Field | Meaning |
|-------|---------|
| `format_version` | `STATE_SNAPSHOT_FORMAT_VERSION` (currently `1`) |
| `scope` | `Full`, `Projects`, `Reviews`, `Verifications` or `Configs` |
| `generated_at`, `ledger_sequence` | When the export ran |
| `project_count` | Total projects registered, for planning pages |
| `start_project_id`, `end_project_id` | Inclusive project ID range covered |
| `projects` | `Project` records in the range |
| `reviews` | Every review of those projects, hidden ones included |
| `verifications` | Every verification record of those projects |
| `config` | Admins, approval threshold, verification duration, fee config, pause flag |
| `state_hash` | SHA-256 of the XDR of the exported data |

Lists outside the chosen scope are empty. `Configs` ignores the project range
and reports `0..0`.

A page covers at most `MAX_SNAPSHOT_PAGE_SIZE` (25) project IDs. `limit = 0`
uses the maximum. Export larger registries page by page, starting the next
page at `end_project_id + 1`, until `end_project_id >= project_count`.

`state_hash` covers only the data, not `generated_at`, so the same state
always hashes the same.

## Publishing a snapshot

```bash
# 1. Export (repeat per page)
stellar contract invoke --id "$CONTRACT" --source admin --network testnet -- \
  export_contract_state_snapshot --admin "$ADMIN" --scope Full \
  --start_project_id 1 --limit 25 > snapshot-1.json

# 2. Hash and pin the exact file
python3 scripts/verify_state_snapshot.py snapshot-1.json --print-sha256
ipfs add --cid-version 1 snapshot-1.json

# 3. Record the CID and file hash on-chain
stellar contract invoke --id "$CONTRACT" --source admin --network testnet -- \
  record_state_snapshot --admin "$ADMIN" --scope Full \
  --start_project_id 1 --limit 25 \
  --ipfs_cid "$CID" --export_sha256 "$FILE_SHA256"
```

`record_state_snapshot` recomputes the snapshot itself and stores the
contract's own `state_hash`, so a record cannot point at data that never
existed. Record the snapshot in the same ledger window as the export; if state
changes in between, the recorded hash will not match the file and the
verifier reports it.

The record stores the ledger timestamp and sequence, the publishing admin, the
CID and both hashes. A `STATE/SNAPSHOT` event is emitted with the new ID.

## Verifying a snapshot

```bash
stellar contract invoke --id "$CONTRACT" --network testnet -- \
  get_state_snapshot --snapshot_id 1 > record-1.json
ipfs get "$CID" -o snapshot-1.json
python3 scripts/verify_state_snapshot.py snapshot-1.json --record record-1.json
```

The verifier checks that the file hash, `state_hash`, scope, format version
and range match the record, and that the file is internally consistent.

To check whether live state has changed since a snapshot, call
`verify_state_snapshot(snapshot_id)`. `matches = false` means state moved on
after the snapshot, not that the snapshot is corrupt.

## Endpoints

| Endpoint | Auth | Returns |
|----------|------|---------|
| `export_contract_state_snapshot(admin, scope, start_project_id, limit)` | admin | `ContractStateSnapshot` |
| `record_state_snapshot(admin, scope, start_project_id, limit, ipfs_cid, export_sha256)` | admin | snapshot ID |
| `get_state_snapshot(snapshot_id)` | none | `Option<StateSnapshotRecord>` |
| `get_state_snapshot_count()` | none | `u64` |
| `verify_state_snapshot(snapshot_id)` | none | `SnapshotVerification` |

Errors: `AdminOnly`, `InvalidCid`, `SnapshotNotFound` (142).
