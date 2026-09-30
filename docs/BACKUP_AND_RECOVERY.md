# Project Backup and Point-in-Time Recovery

This document describes the automated daily snapshot, decentralized backup storage, cryptographic integrity verification, and admin-approved point-in-time recovery system for the Dongle Smart Contract.

---

## 1. Overview & Architecture

The Dongle smart contract manages on-chain project registrations, metadata, and verification statuses on Stellar/Soroban. The backup and recovery system ensures:

1. **Daily Automated Snapshots**: Complete on-chain project registry state is captured daily at 02:00 UTC.
2. **Decentralized Storage**: Snapshots are canonicalized (RFC 8785) and content-addressed via IPFS (CIDv1) and Arweave.
3. **Backup Integrity Verification**: Cryptographic validation using SHA-256 state digests, Merkle tree root hashes, and on-chain project integrity hashes matching `ProjectRegistry::compute_integrity_hash`.
4. **Point-in-Time Recovery**: Ability to revert contract state to any past snapshot, with mandatory **Admin Approval** and multi-signature governance quorum enforcement.

```
┌────────────────────────────────────────────────────────┐
│               Stellar / Soroban Network                │
│             Dongle Smart Contract State                │
└───────────────────────────┬────────────────────────────┘
                            │
              1. Daily Snapshot (02:00 UTC)
                            ▼
┌────────────────────────────────────────────────────────┐
│             project_backup_recovery.py                 │
│  - Reads all project states, stats, & metadata         │
│  - RFC 8785 deterministic JSON canonicalization        │
│  - Computes SHA-256 State Root & Merkle Tree Root      │
│  - Matches on-chain Project Integrity Hash             │
└─────────────┬───────────────────────────┬──────────────┘
              │                           │
  2. Decentralized Pinning       3. Integrity Check
              ▼                           ▼
┌───────────────────────────┐ ┌──────────────────────────┐
│   IPFS (CIDv1) & Arweave  │ │ Cryptographic Check:     │
│ - Pinata / Kubo IPFS Node │ │ - State SHA-256 Matches  │
│ - Decentralized Gateways  │ │ - Merkle Tree Valid      │
│ - Pin Registry Recorded   │ │ - Zero Bitflip Tolerance │
└─────────────┬─────────────┘ └──────────────────────────┘
              │
              │ 4. Disaster / Rollback Trigger
              ▼
┌────────────────────────────────────────────────────────┐
│               Point-in-Time Recovery                   │
│  - Admin selects target snapshot                       │
│  - Generates Restore Manifest & Point-in-Time Diff     │
│  - ENFORCES ADMIN APPROVAL (Quorum / Multi-Sig)        │
│  - Executes audited rollback on-chain                  │
│  - Post-restore cryptographic state verification       │
└────────────────────────────────────────────────────────┘
```

---

## 2. Acceptance Criteria Fulfillment

| Acceptance Criterion | Implementation Details | Verification Command |
|---|---|---|
| **Daily snapshots of project state** | Automated GitHub Actions workflow (`daily_backup.yml`) running daily at 02:00 UTC; CLI `scripts/daily_backup.sh` for local crontab. | `python3 scripts/project_backup_recovery.py snapshot` |
| **Restore to any snapshot (admin approval required)** | Point-in-time diff engine generates an `AdminRestoreManifest`. Requires cryptographic admin approval (or multi-sig quorum if threshold > 1). Restore is rejected without approved status. | `python3 scripts/project_backup_recovery.py restore --manifest <manifest> --admin <admin>` |
| **Backup storage in decentralized manner** | Snapshots are packaged as content-addressed IPFS CIDv1 payloads with support for Pinata, Kubo IPFS daemon, and Arweave transaction logging. | `python3 scripts/project_backup_recovery.py snapshot` |
| **Backup integrity verification** | Recomputes SHA-256 state hash, Merkle tree root, and project-by-project integrity hashes (`project-integrity-v1\|name\|slug\|category\|description`). Fails if any byte is altered. | `python3 scripts/project_backup_recovery.py verify --snapshot <snapshot>` |

---

## 3. Snapshot Data Model & Schema

Each backup snapshot adheres to the formal JSON Schema defined in [`docs/project-backup.schema.json`](./project-backup.schema.json). See [`docs/project-backup.example.json`](./project-backup.example.json) for a complete example.

Key header fields:
- `schemaVersion`: SemVer schema format (currently `1.0.0`).
- `snapshotId`: Unique deterministic ID (e.g. `snap-2026-09-25T02-00-00Z-a1b2c3d4`).
- `ledgerSequence`: Exact Stellar ledger block sequence captured.
- `stateIntegrityHash`: SHA-256 hash across the canonical sorted project array.
- `merkleRoot`: Merkle tree root hash of all project leaf hashes.
- `decentralizedStorage`: Contains the IPFS CIDv1, Arweave TX ID, and gateway URLs.
- `projects`: Complete array of project objects including all metadata, owner, verification status, and on-chain integrity hashes.

---

## 4. Cryptographic Integrity Verification

To protect against bitrot, malicious tampering, or man-in-the-middle attacks, the backup system implements three levels of cryptographic integrity checks:

### 1. State Digest (SHA-256)
All projects are sorted by `id` and formatted using canonical JSON (RFC 8785: sorted keys, compact delimiters, ASCII-safe UTF-8). The SHA-256 digest of this byte sequence must exactly match `stateIntegrityHash`.

### 2. Merkle Tree Root
Each project's metadata hash forms a leaf in a binary Merkle tree. Pairwise hashes are computed up to the root. Modifying or deleting any single project alters the computed root, immediately failing verification.

### 3. On-Chain Project Integrity Hash Alignment
The backup suite matches the exact canonical formula used in `dongle-smartcontract/src/project_registry.rs`:
```
project-integrity-v1|<name>|<slug>|<category>|<description>
```
During verification, the hash of these fields is checked against the stored `integrityHash` for every project in the snapshot.

---

## 5. Point-in-Time Recovery Workflow

Point-in-time recovery allows an administrator to roll back or restore the contract state to any past snapshot.

### Step 1: List Available Recovery Points
```sh
python3 scripts/project_backup_recovery.py list-snapshots
```
Output:
```
Found 3 recovery point(s):
----------------------------------------------------------------------------------------
Snapshot ID                                Timestamp              Ledger     Projects CID
----------------------------------------------------------------------------------------
snap-2026-09-25T02-00-00Z-a1b2c3d4        2026-09-25T02:00:00Z   5241890    2        bafkreihdwdcefgh4d...
snap-2026-09-24T02-00-00Z-c5d6e7f8        2026-09-24T02:00:00Z   5227490    2        bafkreia7g2gg4pux6...
----------------------------------------------------------------------------------------
```

### Step 2: Plan Point-in-Time Restore & Generate Diff Manifest
The `plan-restore` command computes differences between the current contract state and the target snapshot, generating an `AdminRestoreManifest`:
```sh
python3 scripts/project_backup_recovery.py plan-restore \
    --snapshot .backups/snapshots/snap-2026-09-24T02-00-00Z-c5d6e7f8.json \
    --requested-by GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N \
    --reason "Reverting compromised project metadata from ledger 5241890" \
    --output .backups/manifests/restore-manifest.json
```

### Step 3: Admin Approval (Mandatory Governance Check)
Restore cannot proceed without explicit admin approval. In multi-signature configurations (where `admin_approval_threshold > 1`), approvals from multiple administrators are required before `executionStatus` transitions to `approved`:
```sh
python3 scripts/project_backup_recovery.py approve-restore \
    --manifest .backups/manifests/restore-manifest.json \
    --admin GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N \
    --signature "<admin_auth_proof>"
```

### Step 4: Execute Recovery
Once approved, the executor applies the point-in-time rollback:
```sh
python3 scripts/project_backup_recovery.py restore \
    --manifest .backups/manifests/restore-manifest.json \
    --admin GDAMR3SOTO2RJK5QRPNDP2K2OTW247V7OVDJ3Z4R3V47N4TZOQCXJ42N
```
Execution:
1. Re-verifies target snapshot integrity.
2. Applies state updates to reverting project records.
3. Sets `executionStatus: "completed"`.
4. Logs execution timestamp and admin signature.

---

## 6. Automation & Scheduling

### GitHub Actions (`.github/workflows/daily_backup.yml`)
- Triggered automatically every day at 02:00 UTC via GitHub Actions cron.
- Can be manually dispatched (`workflow_dispatch`) from the GitHub UI for on-demand backups before planned upgrades.
- Uploads snapshots to GitHub artifacts (retained for 90 days) and pins to IPFS/Pinata if credentials are configured.

### Linux / Crontab (`scripts/daily_backup.sh`)
For independent operational node backups:
```sh
# Add to crontab:
0 2 * * * /opt/dongle/Dongle-Smartcontract/scripts/daily_backup.sh >> /var/log/dongle_backup.log 2>&1
```

---

## 7. Security & Threat Model

| Threat | Mitigation |
|---|---|
| **Unauthorized State Rollback** | Strict admin authorization check. If multi-sig is enabled, requires $M$-of-$N$ admin approvals in the restore manifest before execution. |
| **Corrupted / Tampered Backup** | Dual-layer verification: Merkle tree root and canonical SHA-256 state hash. Restore aborts immediately if verification fails. |
| **Centralized Storage Outage** | Snapshots are stored in decentralized storage (IPFS & Arweave) and cached in local pin registries. |
| **Concurrent Mutation Race** | Emergency pause (`pause()`) can be initiated prior to point-in-time recovery to guarantee atomic, conflict-free state restoration. |
