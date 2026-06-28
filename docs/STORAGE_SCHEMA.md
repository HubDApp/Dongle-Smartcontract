# Storage Schema Reference

This document describes the persistent storage schema used by the Dongle smart contract, including:

- each storage key and its purpose
- which functions read/write each key
- index consistency rules
- migration notes for future schema evolution

> Source of truth for keys: `dongle-smartcontract/src/storage_keys.rs`  
> Additional extension keys are defined in `ExtensionKey` (used by modules like timelock and fee payment details).

---

## 1) Storage model overview

The contract uses Soroban persistent storage with typed enum keys (`StorageKey`), plus module-specific extension keys (`ExtensionKey`).

High-level groups:

- **Project registry & metadata**
- **Reviews & rating indexes**
- **Verification workflow**
- **Fee configuration & payment flags**
- **Admin/governance**
- **Reports**
- **Collections**
- **Operational/admin extensions** (timelock actions, fee payment details, etc.)

---

## 2) Key catalog (StorageKey)

## Project core

- `Project(u64)`  
  Stores canonical `Project` record by project ID.
- `NextProjectId`  
  Monotonic counter for next project ID.
- `ProjectCount`  
  Total number of projects.
- `ProjectByName(String)`  
  Name uniqueness / lookup index.
- `ProjectBySlug(String)`  
  Slug uniqueness / lookup index.
- `OwnerProjects(Address)`  
  List of project IDs owned by address.
- `OwnerProjectCount(Address)`  
  Cached count of projects per owner.
- `ProjectStats(u64)`  
  Aggregated project rating/statistics.
- `CategoryProjects(String)`  
  Category -> list of project IDs index.
- `ProjectTags(u64)`  
  Tags for project.
- `ProjectLaunchTimestamp(u64)`  
  Launch timestamp metadata.
- `ProjectBountyUrl(u64)`  
  Optional bounty URL.
- `ProjectSocialLinks(u64)`  
  Social links collection.
- `ProjectMaintainers(u64)`  
  Maintainer addresses/identities.
- `ProjectLinkedProjects(u64)`  
  Related/linked project IDs.
- `FeaturedProjects`  
  List of featured project IDs.

## Reviews

- `Review(u64, Address)`  
  Per-project per-reviewer review payload.
- `ProjectReviews(u64)`  
  Project -> list of reviewer addresses index.
- `UserReviews(Address)`  
  Reviewer -> list of reviewed project IDs index.
- `ReviewsEnabled(u64)`  
  Optional override (absent => enabled by default).
- `ReviewReport(u64, Address, Address)`  
  Dedup marker for reporting a review: `(project_id, reviewer, reporter)`.

## Verification

- `Verification(u64)`  
  Current verification state by project.
- `NextVerificationRequestId`  
  Monotonic counter for verification request IDs.
- `VerificationRecord(u64)`  
  Verification request/record by request ID.
- `ProjectVerificationHistory(u64)`  
  Project -> list of verification request IDs.
- `VerificationRenewal(u64)`  
  Active renewal data for project.
- `VerificationRenewalHistory(u64, u32)`  
  Historical renewal entry by `(project_id, renewal_index)`.
- `VerificationRenewalCount(u64)`  
  Renewal counter per project.
- `MinProjectAge`  
  Config for minimum project age requirement.

## Fees / treasury

- `FeeConfig`  
  Global fee settings (token, verification/registration fees, etc.).
- `Treasury`  
  Fee sink address.
- `FeePaidForProject(u64)`  
  Verification payment flag by project.
- `RegistrationFeePaidForAddress(Address)`  
  Registration payment flag by address.

## Admin / governance

- `Admin(Address)`  
  Boolean mapping for admin membership.
- `AdminList`  
  List of all admin addresses.
- `PendingTransfer(u64)`  
  Pending project ownership transfer recipient.

## Reports

- `ProjectReports(u64)`  
  Report list for project.
- `ProjectReportCount(u64)`  
  Cached report count per project.
- `UserReport(u64, Address)`  
  Dedup marker: reporter already reported project.

## Collections

- `Collection(u64)`  
  Collection record by ID.
- `CollectionNameById(u64)`  
  Name cache/index for collection ID.
- `NextCollectionId`  
  Monotonic collection ID counter.
- `CollectionList`  
  Global list of collection IDs.
- `CollectionProjectIds(u64)`  
  List of project IDs in collection.

---

## 3) Extension key usage (non-StorageKey)

Modules also store data under `ExtensionKey` variants (defined in extension key module), including:

- timelock actions and params (`timelock_manager`)
- fee payment details records (`fee_manager`)
- other feature-specific operational records

These keys are part of the schema and must be versioned/migrated with the same discipline as `StorageKey`.

---

## 4) Read/write map by functional area

> This section describes primary read/write behavior observed from current modules.  
> Keep this updated whenever storage logic changes.

## Admin manager / auth / utils

Reads:
- `Admin(Address)` via admin checks (`auth`, `utils`, admin manager paths)
- `AdminList` (admin list operations)

Writes:
- `Admin(Address)` on add/remove admin
- `AdminList` on add/remove admin

Invariants:
- `AdminList` must exactly reflect set of addresses where `Admin(addr) == true`.
- Removing admin must clear both mapping and list membership.
- At least one admin safety rule (if enforced in code) must be preserved through migrations.

## Fee manager

Reads:
- `FeeConfig`
- `Treasury`
- `FeePaidForProject(project_id)`

Writes:
- `FeeConfig` (admin updates)
- `Treasury` (admin updates or fee setup paths)
- `FeePaidForProject(project_id)` set on successful payment
- `FeePaidForProject(project_id)` removed when consumed

Related extension writes:
- `ExtensionKey::FeePaymentDetails(project_id)`

Invariants:
- Payment flag is set only after transfer succeeds.
- Consuming payment must remove flag atomically with business operation gating.
- `FeeConfig.token` and fee amount semantics must remain internally consistent.

## Project registry

Reads:
- `NextProjectId`, `ProjectCount`
- `ProjectByName`, `ProjectBySlug` (uniqueness)
- `OwnerProjects(owner)`, `OwnerProjectCount(owner)`
- `CategoryProjects(category)`
- project metadata keys (`ProjectTags`, links, maintainers, etc.)

Writes:
- `Project(id)`, increment `NextProjectId`
- increment/update `ProjectCount`
- set/unset `ProjectByName(name)`, `ProjectBySlug(slug)`
- append/remove in `OwnerProjects(owner)`, update `OwnerProjectCount(owner)`
- maintain `CategoryProjects(category)` membership
- write metadata adjunct keys (tags/social/launch/etc.)

Invariants:
- Canonical existence: `Project(id)` is source of truth.
- Name/slug indexes must point to existing project IDs only.
- Owner/category indexes must not contain duplicates.
- Cached counts must match vector lengths where both are used.

## Review registry / rating

Reads:
- `Review(project_id, reviewer)`
- `ProjectReviews(project_id)`, `UserReviews(reviewer)`
- `ProjectStats(project_id)`
- `ReviewsEnabled(project_id)`

Writes:
- create/update/remove `Review(project_id, reviewer)`
- maintain `ProjectReviews(project_id)` and `UserReviews(reviewer)` bidirectional indexes
- update `ProjectStats(project_id)` aggregates
- optional writes to `ReviewsEnabled(project_id)`

Invariants:
- `Review(project, reviewer)` existence must match membership in both indexes.
- On add: both indexes gain entry; on delete: both indexes lose entry.
- Rating aggregate (`ProjectStats`) must be recomputed/updated from exact delta rules.
- Enforce configured index caps (`MAX_REVIEWS_PER_PROJECT`, `MAX_REVIEWS_PER_USER`).

## Verification manager

Reads:
- `Verification(project_id)`
- `NextVerificationRequestId`
- `VerificationRecord(request_id)`
- `ProjectVerificationHistory(project_id)`
- `VerificationRenewal*`
- `MinProjectAge`
- fee/payment keys as preconditions where required

Writes:
- create/update `Verification(project_id)`
- increment `NextVerificationRequestId`
- write `VerificationRecord(request_id)`
- append to `ProjectVerificationHistory(project_id)`
- write/update renewal keys and counts

Invariants:
- Request ID counter must be monotonic and never reused.
- Every created verification record ID should be discoverable from project history.
- Renewal count must equal number of historical renewal entries.

## Report registry

Reads:
- `ProjectReports(project_id)`
- `ProjectReportCount(project_id)`
- `UserReport(project_id, reporter)`

Writes:
- append `ProjectReports(project_id)`
- set `UserReport(project_id, reporter)` for dedup
- set/update `ProjectReportCount(project_id)`
- clear path removes all above keys and dedup marks

Invariants:
- `ProjectReportCount` equals `ProjectReports.len()`.
- For each stored report by reporter R, dedup key `UserReport(project_id, R)` must exist.
- Clearing reports must remove both aggregate and dedup keys.

## Collection registry

Reads:
- `NextCollectionId`, `CollectionList`
- `Collection(id)`, `CollectionNameById(id)`
- `CollectionProjectIds(id)`

Writes:
- create: set collection record + name cache + empty project list + append list + increment counter
- update: modify `Collection(id)` and `CollectionNameById(id)`
- delete: remove collection keys and remove id from `CollectionList`

Invariants:
- `CollectionList` contains every existing collection ID exactly once.
- `CollectionNameById(id)` matches `Collection(id).name`.
- `CollectionProjectIds(id)` exists iff collection exists (or is safely treated as empty).

## Timelock manager (extension keys)

Reads/Writes:
- `ExtensionKey::TimelockAction(id)`
- `ExtensionKey::Timelock*Params(id)`
- timelock action ID list / next ID extension keys

Invariants:
- Timelock action IDs are monotonic.
- For each scheduled action, corresponding params key must exist.
- Executed/cancelled terminal transitions are one-way.

---

## 5) Index consistency rules (authoritative checklist)

When mutating state, keep these consistency rules true in the same transaction:

1. **Primary-first rule**  
   Canonical object key (`Project`, `Review`, `Collection`, etc.) is written/removed with all derived indexes atomically.

2. **Bidirectional index symmetry**  
   For review links:
   - if `Review(project, reviewer)` exists -> reviewer in `ProjectReviews(project)` and project in `UserReviews(reviewer)`.
   - if removed -> remove from both.

3. **Count mirrors list length**  
   Keys like `OwnerProjectCount`, `ProjectReportCount`, renewal counters must match actual stored list/history length.

4. **Uniqueness index hygiene**  
   Name/slug uniqueness indexes must be updated on create/update/delete and never point to missing entities.

5. **No duplicate IDs in list indexes**  
   Before append, ensure not already present (or ensure call path guarantees uniqueness).

6. **Delete cleanup completeness**  
   Deleting/clearing an entity removes all dependent dedup/index keys (e.g., report dedup entries).

7. **Monotonic counters only increment**  
   `Next*Id` counters are never decremented or reused.

8. **Default-by-absence semantics preserved**  
   For keys like `ReviewsEnabled`, absence has meaning; migrations must preserve behavior.

---

## 6) Migration notes for future schema changes

## Versioning strategy

- Introduce explicit schema version key (recommended: `ExtensionKey::SchemaVersion`).
- On contract init/upgrade, run idempotent migration routines:
  - `if version < N { migrate_to_N(); set version = N; }`
- Never rely on one-shot assumptions; migrations must be safe to re-enter after partial failure.

## Safe migration workflow

1. **Additive phase first**
   - Add new keys/indexes while preserving old read paths.
2. **Backfill phase**
   - Iterate canonical records and reconstruct derived indexes/caches.
3. **Dual-read / dual-write (if needed)**
   - Temporarily read old+new, write both.
4. **Cutover phase**
   - Switch reads to new schema once backfill verified.
5. **Cleanup phase**
   - Remove deprecated keys in a later version (optional but recommended).

## Migration invariants to verify

After migration, validate:

- Every `Project(id)` has consistent owner/category/name/slug indexes.
- Every `Review` has both directional index entries.
- Count caches equal list lengths.
- No orphan uniqueness index entries.
- Verification history links and counters are consistent.
- Collection list/name/indexes are synchronized.

## Counter and ID migrations

- Preserve existing `Next*Id` values exactly.
- If reconstructing from data, set `Next*Id = max(existing_ids)+1`.
- Never reset counters during migration.

## Handling defaulted/optional keys

- Preserve existing absence semantics (`unwrap_or` defaults in read paths).
- For newly required keys, define deterministic defaults and backfill explicitly.

## Recommended test coverage for each migration

- **Pre-state fixture** from old schema.
- Run migration.
- Assert all invariants in section 5.
- Assert business operations still succeed (register/update/review/verify/report/collection ops).
- Add property/invariant tests where feasible for index symmetry and count correctness.

---

## 7) Operational notes

- TTL bump logic in `storage_manager` should include newly introduced critical keys where needed.
- If adding new list indexes, define hard caps and pagination behavior up front.
- Any new dedup key must have a corresponding cleanup path.
- Keep this document updated in the same PR as storage schema changes.

---

## 8) Maintainer checklist (PR gate for storage changes)

For any PR modifying storage:

- [ ] Updated `storage_keys.rs` / extension keys with docs
- [ ] Updated this `docs/STORAGE_SCHEMA.md`
- [ ] Documented read/write paths for changed keys
- [ ] Added/updated invariant tests
- [ ] Added migration logic (or documented why not needed)
- [ ] Verified no orphaned indexes on delete/update paths
- [ ] Verified counter monotonicity