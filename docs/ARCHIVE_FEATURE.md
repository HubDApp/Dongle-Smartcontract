# Project Archive and Reactivation

Project archiving hides a project from discovery lists without deleting its
data. The project owner or a contract administrator can later reactivate it.
Both transitions preserve the project record, update its timestamp, extend its
storage lifetime, and emit an event for off-chain consumers.

This guide reflects the current implementation under
`dongle-smartcontract/src`. It replaces four overlapping root-level documents
whose API details, errors, test counts, and authorization rules had drifted.

## Public API

| Method | Authorized caller | Result |
|--------|-------------------|--------|
| `archive_project(project_id, caller)` | Project owner or administrator | Sets `archived` to `true` |
| `reactivate_project(project_id, caller)` | Project owner or administrator | Sets `archived` to `false` |
| `get_project(project_id)` | Public | Returns the project even when archived |

The contract entrypoints are defined in
[`lib.rs`](../dongle-smartcontract/src/lib.rs), and their behavior is
implemented by
[`ProjectRegistry`](../dongle-smartcontract/src/project_registry.rs).

## Archive Operation

`archive_project` requires authentication from `caller`. It then:

1. loads the project or returns `ProjectNotFound`;
2. verifies that the caller is the owner or a registered administrator;
3. rejects a project whose `archived` flag is already `true`;
4. sets `archived` to `true` and updates `updated_at` to ledger time;
5. writes the project and extends its storage lifetime; and
6. emits `ProjectArchivedEvent`.

```rust
client.archive_project(&project_id, &owner);
assert!(client.get_project(&project_id).unwrap().archived);
```

The registry also exposes an internal authorization-bypassing helper used by
trusted contract flows after they have performed their own authorization. It is
not a separate public contract entrypoint for ordinary clients.

## Reactivation Operation

`reactivate_project` uses the same owner-or-administrator authorization. It:

1. loads the project or returns `ProjectNotFound`;
2. rejects callers who are neither owner nor administrator;
3. rejects a project whose `archived` flag is already `false`;
4. sets `archived` to `false` and updates `updated_at`;
5. writes the project and extends its storage lifetime; and
6. emits `ProjectReactivatedEvent`.

```rust
client.reactivate_project(&project_id, &owner);
assert!(!client.get_project(&project_id).unwrap().archived);
```

Repeated archive and reactivation cycles preserve the rest of the `Project`
record, including ownership, metadata, reviews, and verification references.

## Errors

| Error | Scenario |
|-------|----------|
| `ProjectNotFound` | The project ID does not exist |
| `Unauthorized` | The caller is neither the owner nor an administrator |
| `AlreadyArchived` | An archive call targets an archived project |
| `ProjectNotArchived` | A reactivation call targets an active project |

Older guides used `ProjectAlreadyArchived`, but the current enum and registry
use `AlreadyArchived`. Consumers should use the generated contract interface
and canonical error reference.

## Discovery and Direct Access

Archived projects remain in persistent storage and can still be returned by
direct lookup, including `get_project`. They are filtered out of these current
discovery paths:

- `get_projects_by_owner`
- `list_projects`
- `list_projects_by_status`
- `list_projects_by_category`
- `list_projects_by_tag`
- `list_projects_sorted`

Reactivation makes the project eligible for those listings again. Each listing
applies its normal pagination and filtering rules in addition to the archive
check.

Archive is therefore a visibility transition, not deletion. Clients must not
treat a missing list result as proof that a project ID does not exist.

## Storage Model

Archive status is the `archived: bool` field on the existing `Project` value.
No separate archive record or index is created. New projects initialize this
field to `false`.

Both transitions write the project at `StorageKey::Project(project_id)` and call
the project TTL extension helper. Related reviews and verification data are not
rewritten.

The current `Project` type is defined in
[`types.rs`](../dongle-smartcontract/src/types.rs).

## Events

| Event | Topic | Actor field |
|-------|-------|-------------|
| `ProjectArchivedEvent` | `PROJECT`, `ARCHIVED`, `project_id` | `archived_by` |
| `ProjectReactivatedEvent` | `PROJECT`, `RESTORED`, `project_id` | `caller` |

Both events include the project ID and current ledger timestamp. The restored
topic uses `RESTORED`, even though the Rust event type is named
`ProjectReactivatedEvent`.

Definitions and publishers live in
[`events.rs`](../dongle-smartcontract/src/events.rs).

## Authorization Notes

- The caller address must authorize the public archive or reactivation call.
- A project owner can change only their own project's state.
- A registered administrator can archive or reactivate any project.
- Event actor fields identify the address that performed the transition, which
  can be the owner or an administrator.

Off-chain interfaces should make administrator-initiated transitions visible
to the project owner and retain the corresponding event for audit purposes.

## Test Coverage

The focused suites contain 25 tests:

- 19 tests in
  [`tests/archive.rs`](../dongle-smartcontract/src/tests/archive.rs) for owner
  behavior, errors, discovery filtering, timestamps, lifecycle cycles, and data
  preservation; and
- 6 tests in
  [`tests/archival.rs`](../dongle-smartcontract/src/tests/archival.rs) for owner
  and administrator authorization plus listing behavior.

Authorization-matrix, cleanup, duplicate-dispute, and event suites exercise
additional archive paths. Run the focused tests from the workspace root with:

```bash
cargo test -p dongle-contract archive
```

## Operational Notes

- Archive and reactivation are constant-size writes to the project record.
- Discovery functions still scan or read their normal indexes, then skip
  archived entries.
- Archiving does not remove reviews, contracts, verification data, or project
  metadata.
- Direct lookup remains available so administrators, owners, and indexers can
  inspect archived projects.
- Event consumers should listen for both `ARCHIVED` and `RESTORED` topics to
  maintain a current visibility state.
