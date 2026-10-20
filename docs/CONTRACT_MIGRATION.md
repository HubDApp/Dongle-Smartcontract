# Contract Version Migration

Issue #935. How to export data from a deployed contract and check it before
importing it into a new contract version.

For changing the storage schema inside one deployment, see
[STORAGE_MIGRATION_GUIDE.md](STORAGE_MIGRATION_GUIDE.md). For single-project
moves, see [PROJECT_DATA_MIGRATION.md](PROJECT_DATA_MIGRATION.md).

## Bundles

`export_for_migration(admin, version, start_project_id, limit)` is admin-only
and returns a `MigrationBundle`:

- `metadata`: format version, source contract address and `CONTRACT_VERSION`,
  target `version`, exporting admin, timestamp and ledger, project range and
  record counts, `next_project_id`, and a `content_hash`.
- `projects`, `reviews` (hidden included), `verifications`: every record for
  the project IDs in the page.
- `config`: admins, approval threshold, verification duration, fee config and
  pause flag. Every page carries it.

A page covers at most `MAX_SNAPSHOT_PAGE_SIZE` (25) project IDs; `limit = 0`
uses the maximum. Follow `metadata.next_project_id` until it is `None`.

`content_hash` is the SHA-256 of the XDR of `(format_version, target_version,
start_project_id, end_project_id, projects, reviews, verifications, config)`.

Each export emits a `MIGRATE/EXPORT` event.

## Versioning

`metadata.format_version` describes the bundle layout. It is picked from the
target version; only format `1` exists today. When the layout changes, add a
new format constant, add it to `SUPPORTED_MIGRATION_FORMATS`, and update
`MigrationExport::format_for_target` and `scripts/validate_migration_bundle.py`.

Target versions must be `MAJOR.MINOR.PATCH`. Export rejects:

- malformed versions with `InvalidInput`
- downgrades below `CONTRACT_VERSION`, or targets more than one major version
  ahead, with `IncompatibleMigrationTarget` (143)

## Validation

`validate_migration_bundle(bundle, target_version)` is read-only and returns a
`MigrationCompatibilityReport`. `compatible` is `true` only when `issues` is
empty. Possible issues:

| Issue | Meaning |
|-------|---------|
| `UnsupportedFormatVersion` | Bundle layout unknown to this contract |
| `TargetVersionMismatch` | Bundle was prepared for another version |
| `InvalidVersionString` | A version is not `MAJOR.MINOR.PATCH` |
| `DowngradeNotSupported` | Target is older than the source |
| `MajorVersionJumpTooLarge` | Target skips a major version |
| `RecordCountMismatch` | List lengths differ from metadata counts |
| `RecordOutOfRange` | A record belongs to a project outside the page |
| `ContentHashMismatch` | Bundle contents were changed after export |

Call it on the source contract (or a new contract that keeps this endpoint)
for every page.

`scripts/validate_migration_bundle.py` runs offline over all pages at once.
It repeats the metadata and version checks and confirms the pages cover
project IDs `1..project_count` with no gaps or overlaps. It cannot recompute
`content_hash`.

## Procedure

1. **Freeze writes.** Pause the source contract (`pause`) so state cannot
   change between pages.
2. **Export every page.**
   ```bash
   start=1; page=1
   while :; do
     stellar contract invoke --id "$OLD" --source admin --network "$NET" -- \
       export_for_migration --admin "$ADMIN" --version "$TARGET" \
       --start_project_id "$start" --limit 25 > "bundle-$page.json"
     next=$(jq -r '.metadata.next_project_id // empty' "bundle-$page.json")
     [ -z "$next" ] && break
     start=$next; page=$((page + 1))
   done
   ```
3. **Validate.**
   ```bash
   python3 scripts/validate_migration_bundle.py bundle-*.json --target-version "$TARGET"
   for f in bundle-*.json; do
     stellar contract invoke --id "$OLD" --network "$NET" -- \
       validate_migration_bundle --bundle-file-path "$f" --target_version "$TARGET"
   done
   ```
   Stop if any report has `compatible: false`.
4. **Archive.** Pin the bundles to IPFS and record a full state snapshot
   (`docs/STATE_SNAPSHOTS.md`) so the pre-migration state stays auditable.
5. **Deploy and import.** Deploy the new version, initialize it with the
   admins from `config`, and import each page with the new contract's import
   entry point.
6. **Verify.** Compare project, review and verification counts on the new
   contract with the bundle metadata, and spot-check records.
7. **Cut over.** Point clients at the new contract. Keep the old one paused,
   not deleted, until the migration is signed off.

## Rollback

The source contract is never modified by export. To roll back, unpause it and
point clients back at it.
