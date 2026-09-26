# Data Retention Policy

This policy defines default retention periods for inactive project data and the
operational requirements for configuring and purging it. Durations are defaults,
not legal advice; the data controller must confirm jurisdiction-specific
obligations before deployment.

## Default Schedule

| Entity type | Inactivity starts | Default retention | Purge scope |
|-------------|-------------------|-------------------|-------------|
| Active project profile | Not applicable while active | Retained while active | None |
| Archived project profile | `archived_at` (the archive transition time) | 2 years | Project metadata and indexes; retain only a non-identifying purge record |
| Review | `updated_at` (or `created_at` for an unchanged review) | 2 years | Move out of active storage, then follow the archived-review period |
| Compact archived review | `archived_at` | 90 days | Remove the compact on-chain record and its enumeration index |
| Terminal verification evidence and history | Final decision time; use request time for legacy records without a decision | 7 years | Remove evidence references and eligible records; retain aggregate audit counts only |
| Resolved reports and disputes | Resolution time | 1 year | Remove reporter-provided content and per-user references |

Activity that changes an entity's content or state resets that entity's
inactivity clock. Merely reading an entity does not reset it. Pending
verification requests, active projects, and unresolved reports are not eligible
for inactivity purging. Re-activating an archived project cancels its archived
project clock.

## Configuration

Retention is configured independently for each entity type, in seconds. The
configured value applies to the entity's inactivity timestamp, not its storage
TTL. Lowering a period may make existing records immediately eligible; raising
it does not restore data already purged. Configuration changes should be made
through the contract's multi-admin governance path, announced before taking
effect, and recorded with the previous value, new value, effective time, and
approving governance action.

Every entity type must have a defined default. A zero duration must not silently
mean "never expire"; disabling purging requires an explicit, audited policy
state so configuration mistakes cannot retain data indefinitely.

## Purge Operation

Soroban contracts do not run timers or background jobs. Automatic purging
therefore requires a scheduled off-chain keeper that submits permissionless,
bounded purge transactions. The keeper should run at least daily, retry failed
batches, and report backlog and failures. Purge calls must re-check the current
policy and inactivity timestamp on-chain, be safe to retry, and process a
bounded number of records per transaction. A user or project becoming active
before execution must prevent the purge.

The current project archive operation only hides a project and is reversible;
it does not delete project data. The existing review archival and verification
history cleanup operations are also separate from this policy. These APIs do
not currently implement the schedule or a general-purpose retention keeper.
The schedule above is the target behavior for that implementation.

## Purge Audit

Each successful purge batch must emit an append-only audit event containing the
entity type, number of records removed, policy version, retention cutoff, purge
timestamp, and transaction/keeper identifier. It must not include record
contents, names, CIDs, owner or reviewer addresses, or other direct identifiers.
Failed and partially completed batches must be observable to the keeper but
must not be reported as successful purges. Off-chain audit consumers should
retain and monitor these events independently of the contract's expiring
persistent storage.

## Privacy and GDPR Limitations

Public-ledger writes and emitted events are immutable. A contract cannot erase
past transactions, event history, validator snapshots, or copies held by third
parties. Consequently, on-chain deletion is not by itself a guarantee of GDPR
erasure or compliance.

- Do not put names, email addresses, legal identities, or unredacted personal
  data in project fields, review content, events, or metadata CIDs.
- Treat public addresses and stable record identifiers as potentially
  personal or linkable data; omitting a name does not make data anonymous.
- Keep substantive documents off-chain. On a valid erasure request, remove
  controlled copies and coordinate unpinning/deletion with the storage provider
  where possible; understand that public IPFS content may persist elsewhere.
- Use the purge audit event only for minimal operational evidence. Keep any
  identity-to-record mapping off-chain, access-controlled, and only as long as
  necessary.
- Provide a documented process for access, correction, objection, and erasure
  requests, including a way to remove or sever off-chain references when
  on-chain data cannot be changed.

The controller remains responsible for defining the lawful basis, validating
retention periods against applicable law, handling data-subject requests, and
documenting residual risk before enabling this policy.