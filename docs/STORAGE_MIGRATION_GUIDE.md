# Storage Schema Migration Guide

This guide describes how to evolve the contract's persistent storage schema safely across deployments.

## Why storage migrations matter

Persistent Soroban storage survives contract upgrades and deployments. Storage keys are defined in [`dongle-smartcontract/src/storage_keys.rs`](../dongle-smartcontract/src/storage_keys.rs). A key's identity is determined by its serialized key value, while the value stored under that key is decoded according to the type expected by the contract code.

Adding a new key is normally backward-compatible because existing keys remain untouched. Changing the value type or meaning of an existing key is different: old on-chain data may no longer decode into the new type, may have a different semantic meaning, or may be impossible to read safely.

For that reason, treat storage schema changes as migrations rather than as ordinary refactors.

## Schema versioning policy

When changing persistent storage, use an explicit schema version for the migration plan and keep a record of the deployed schema version. A version can be represented by a dedicated storage key or by deployment metadata, provided the migration process can unambiguously determine which schema an existing instance uses.

Recommended versioning rules:

1. **Document every schema version.** Record the keys introduced, removed, renamed, or changed and the expected value type for each version.
2. **Never silently reinterpret an existing key.** If a value's type or meaning changes, introduce a new versioned key instead of assuming the old value can be decoded as the new type.
3. **Keep the old representation during migration.** Read the old key, validate it, convert it, and write the converted value to the new key.
4. **Make migrations explicit and repeatable.** A migration should have a clear precondition, target version, validation checks, and completion state.
5. **Do not delete old data prematurely.** Retain the old key until the new representation has been populated and verified, and until the project has established that the old representation is no longer required.

## Adding a new storage key

Adding a new variant to `StorageKey` or `ExtensionKey` is generally the safest schema change. Existing records are not automatically rewritten, so existing keys continue to represent their old values.

For a new key:

1. Add the new key variant.
2. Document the key and its value type in the storage schema documentation.
3. Define the default behavior when the key is absent.
4. Deploy the code that understands the new key.
5. Populate the key lazily or through an explicit migration if existing records need a value immediately.

For example, adding `ProjectRegion(u64)` does not require rewriting every existing `Project(u64)` record. The contract can initially treat a missing region value as "not set" and populate it when appropriate.

## Changing an existing value type

Changing the type stored under an existing key is **not** a safe in-place change. For example, if an old deployment stores:

```text
ProjectStats(project_id) -> OldStats
```

and a new deployment expects:

```text
ProjectStats(project_id) -> NewStats
```

then existing values may fail to decode or may not have the fields required by `NewStats`.

Instead, use a new key or versioned representation:

```text
ProjectStats(project_id)       -> OldStats
ProjectStatsV2(project_id)     -> NewStats
```

The migration should then:

1. Detect the current schema version.
2. Read the old value using the old type.
3. Validate the old value before conversion.
4. Convert it to the new representation.
5. Write the new value under the new key.
6. Verify that the new value can be read using the new type.
7. Mark the migration complete only after the conversion succeeds.

## Changing the meaning of an existing key

A type-compatible change can still be a schema migration if the meaning changes. For example, changing a stored integer from "seconds" to "milliseconds" without changing its Rust type would leave the data decodable but semantically incorrect.

Treat semantic changes exactly like type changes: introduce a new representation, convert existing data deliberately, and document the conversion rule.

## Recommended migration sequence

For a deployment that changes persistent storage, use this sequence:

```text
1. Identify the current schema version
          |
          v
2. Read and validate existing data
          |
          v
3. Convert old representation -> new representation
          |
          v
4. Write the new keys
          |
          v
5. Verify representative and boundary records
          |
          v
6. Record the new schema version
          |
          v
7. Retire old keys only after the migration is proven safe
```

If a migration can affect many records, prefer bounded batches or another resumable mechanism rather than assuming the entire dataset can be migrated in one transaction. Record enough progress to safely resume after an interruption.

## Example migration record

For each release that changes persistent storage, maintain a short migration record such as:

| Schema | Change | Migration | Compatibility |
|---|---|---|---|
| `v1` | Original `ProjectStats` representation | None | Baseline |
| `v2` | Added `ProjectRegion(u64)` | Missing values default to unset; existing projects may be populated lazily | Backward-compatible |
| `v3` | Changed project statistics representation | Read `v1`/`v2` stats, convert to the new representation, write versioned keys, verify, then mark `v3` complete | Requires migration |

The exact version numbers and key names should match the deployed contract rather than this example.

## Testing before deployment

Before deploying a schema change:

- Test against a fixture containing data from the previous deployment.
- Verify every key that the new code reads still decodes with the expected type.
- Test missing-key behavior for newly introduced fields.
- Test migration conversion with normal, boundary, and malformed legacy values.
- Test interrupted and resumed migrations when the migration is not atomic.
- Verify that a second migration attempt is rejected or safely treated as already complete.
- Keep a backup/export or other recovery procedure appropriate to the deployment environment before performing irreversible cleanup.

## Reference

The authoritative list of persistent key variants is [`storage_keys.rs`](../dongle-smartcontract/src/storage_keys.rs). Keep this guide and the storage-key definitions synchronized whenever the persistent schema changes.

> **Important:** A documentation migration plan does not itself migrate on-chain data. Before deploying a schema-changing contract version, verify the actual migration mechanism supported by the deployed contract and test it against representative existing state.
