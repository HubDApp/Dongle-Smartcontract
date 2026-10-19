# Collection Immutable Snapshots Architecture (#908)

## Overview

The Collection Immutable Snapshots feature allows curators, community owners, and dApp consumers to create permanent, verifiable, point-in-time snapshots of collection state. Each snapshot captures:
- Collection metadata (ID, name, description, owner, visibility)
- All associated projects and their metadata at snapshot time
- A sequential, monotonically increasing version number
- An IPFS Content Identifier (CID) referencing the published immutable snapshot document

Snapshots can be compared on-chain or off-chain to audit membership changes (added, removed, and retained projects) across versions.

## On-Chain Contract Interface

### Snapshot Creation
```rust
pub fn snapshot_collection(
    env: Env,
    caller: Address,
    collection_id: u64,
    ipfs_cid: Option<String>,
) -> Result<CollectionSnapshot, ContractError>
```
- Authenticates the caller, ensuring caller is either the collection owner or a system administrator.
- Increments the collection's version counter (`v1`, `v2`, ...).
- Validates the provided IPFS CID format (CIDv0 `Qm...` or CIDv1 `bafy...`) or sets the canonical deterministically hashed snapshot CID.
- Stores the snapshot immutably on-chain and emits `COL_SNAP` event.

### Historical Retrieval
```rust
pub fn get_collection_snapshot(env: Env, collection_id: u64, version: u32) -> Option<CollectionSnapshot>
pub fn get_latest_collection_snapshot(env: Env, collection_id: u64) -> Option<CollectionSnapshot>
pub fn list_collection_snapshots(env: Env, collection_id: u64, start_index: u32, limit: u32) -> Vec<CollectionSnapshot>
```

### Version Comparison
```rust
pub fn compare_collection_snapshots(
    env: Env,
    collection_id: u64,
    version_a: u32,
    version_b: u32,
) -> Result<CollectionSnapshotComparison, ContractError>
```
Returns a `CollectionSnapshotComparison` struct containing:
- `collection_id`: target collection
- `version_a`: base version
- `version_b`: target version
- `cid_a` & `cid_b`: IPFS CIDs for both versions
- `added_projects`: project IDs present in `version_b` but absent from `version_a`
- `removed_projects`: project IDs present in `version_a` but removed from `version_b`
- `unchanged_projects`: project IDs maintained across both versions

## Off-Chain Snapshot Generation and IPFS Publishing

Off-chain clients or indexers can export the complete state of a collection by fetching:
1. The collection record via `get_collection`
2. All project records via `list_collection_projects` / `get_project`
3. Assembling the JSON payload compliant with [collection-snapshot.schema.json](file:///home/fredrick/Desktop/Drips%20Wave/Dongle-Smartcontract/docs/collection-snapshot.schema.json)
4. Publishing the JSON file to IPFS (via IPFS Kubo, Pinata, Web3.Storage, or Helia) to obtain the immutable CID
5. Calling `snapshot_collection(caller, collection_id, Some(ipfs_cid))` to persist the verifiable record on-chain.
