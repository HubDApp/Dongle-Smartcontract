# Reserved Project Names

The contract keeps an admin-managed list of **reserved project names**. A
reserved name cannot be used as the `name` of any project, on registration or
on a later name change. The feature exists to stop squatting and impersonation
of well-known brands, protocols, and the platform itself.

Relevant source: `dongle-smartcontract/src/project_registry.rs`
(`add_reserved_name`, `remove_reserved_name`, `get_reserved_names`,
`is_name_reserved`, `check_reserved_name`).

## Why reserve a name

- **Impersonation / phishing.** Prevent a malicious project from registering
  as `Stellar`, `USDC`, `Soroban`, a partner protocol, or the platform brand
  and then trading on that trust.
- **Trademark protection.** Hold names on behalf of partners until they
  onboard.
- **Operational placeholders.** Keep internal or future names out of the
  public namespace.

Reserving a name is a governance action: it should be backed by an off-chain
policy (e.g. a published blocklist rationale) and is recorded in the admin
action log.

## Matching semantics

- **Case-insensitive.** `check_reserved_name` lowercases both the candidate
  and each list entry before comparing, so reserving `Stellar` also blocks
  `stellar` and `STELLAR`.
- **Exact string match after lowercasing.** There is no substring, fuzzy, or
  homoglyph matching. `Stellar Foundation` is *not* blocked by reserving
  `Stellar`. Reserve each variant you want to block.
- The reserved-name check is independent of the separate exact-match name
  uniqueness check (`ProjectAlreadyExists`) and the normalized duplicate-name
  check (`DuplicateProjectName`).

## Public / read API

| Function | Auth | Description |
| --- | --- | --- |
| `get_reserved_names() -> Vec<String>` | none | Full reserved list, as stored (original casing). |
| `is_name_reserved(name: String) -> bool` | none | `true` if `name` is reserved (case-insensitive). |

Front-ends should call `is_name_reserved` during name-input validation so the
user sees the failure before submitting a transaction.

## Admin API

| Function | Auth | Behaviour |
| --- | --- | --- |
| `add_reserved_name(admin, name)` | admin | Adds `name`. No-op (returns `Ok`) if an equal name (case-insensitive) is already reserved. Emits `RSVD_ADD`, records `ReservedNameAdded` in the admin action log. |
| `remove_reserved_name(admin, name)` | admin | Removes the matching entry (case-insensitive). No-op (returns `Ok`) if not present. Emits `RSVD_REM`, records `ReservedNameRemoved`. |

Both admin calls go through `require_admin_auth`, so under multi-sig they must
be authorized by the configured admin quorum.

`add` / `remove` are intentionally **idempotent** — repeating a call does not
error. Callers that need to know whether a change actually happened should
read `get_reserved_names` (or watch the events) before and after.

## Enforcement paths

The reserved-name check runs in **every** path that sets or changes a project
name:

| Path | Where | Error on hit |
| --- | --- | --- |
| `register_project` | `validate_registration_fields` → `check_reserved_name(params.name)` | `ReservedName` (43) |
| `update_project` (changing the `name` field) | field-update handler → `check_reserved_name(new_value)` | `ReservedName` (43) |

Ownership transfer does not change the name, so it needs no check. There is no
other way to set a project name.

### Effect on already-registered names

Reserving a name does **not** retroactively remove or rename existing
projects that already hold it. It only blocks future registrations and future
renames *to* that name. To reclaim an already-taken name, governance must use
the normal moderation / ownership tooling separately.

## Events

Reserved-name changes publish under the `CONFIG` topic namespace:

| Action | Topic | Payload struct | Fields |
| --- | --- | --- | --- |
| Added | `(Symbol("CONFIG"), Symbol("RSVD_ADD"))` | `ReservedNameAddedEvent` | `name: String`, `admin: Address`, `timestamp: u64` |
| Removed | `(Symbol("CONFIG"), Symbol("RSVD_REM"))` | `ReservedNameRemovedEvent` | `name: String`, `admin: Address`, `timestamp: u64` |

Indexers can rebuild the current reserved list from the event stream, or just
read `get_reserved_names`.

## Operational guidance for admins

- Maintain the canonical policy list off-chain; treat the on-chain list as the
  enforced subset.
- Add high-value brand and platform names before public launch.
- Because matching is exact-after-lowercasing, add the obvious spelling
  variants (spaced, hyphenated, `.xyz`-suffixed, etc.) for names you really
  care about.
- Removing a name frees it immediately for the next registrant — coordinate
  removals with the party you are handing the name to.
