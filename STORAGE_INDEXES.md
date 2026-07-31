# Storage Index Size Strategy

On-chain indexes in Dongle are `Vec`-backed storage entries keyed by live `StorageKey` / `ExtensionKey` variants in `dongle-smartcontract/src/storage_keys.rs`. Each index grows on write (`push_back`) and is trimmed on delete. Without caps, a single user or project could accumulate unbounded entries and exhaust Soroban storage budgets.

This document defines **maximum expected index sizes**, **write-time enforcement**, and **read pagination** for index consumers (frontends and off-chain indexers). It is reconciled against the live key enums — dead scaffolding is listed only to exclude it from the scheme.

Constants live in `dongle-smartcontract/src/constants.rs`.

## Pagination (read path)

All paginated list endpoints clamp `limit` to `MAX_PAGE_LIMIT` (**100**). When `limit` is `0` or greater than 100, the effective limit is 100.

| Parameter | Behavior |
|-----------|----------|
| `limit = 0` | Treated as `MAX_PAGE_LIMIT` (100) on most endpoints |
| `limit > 100` | Clamped to 100 |
| `start_id` | Project ID cursor (`list_projects`, `list_projects_by_status`) |
| `start_index` | Zero-based offset into an index Vec or sorted result |

Indexers should page through large indexes using these parameters rather than assuming a single call returns all entries.

## Primary indexes (live)

### Owner projects — `StorageKey::OwnerProjects(Address)` → `Vec<u64>`

| Property | Value |
|----------|-------|
| **Max size** | `MAX_PROJECTS_PER_USER` = **50** |
| **Enforced on** | `register_project`, `accept_transfer`, `approve_claim_request` |
| **Error** | `MaxProjectsExceeded` (30) |
| **Read API** | `get_projects_by_owner`, `get_owner_project_count` |
| **Pagination** | Not required — index is bounded at 50; full fetch is safe |
| **Notes** | Archived projects remain in the index; `get_projects_by_owner` filters them at read time |

### Project reviews — `StorageKey::ProjectReviews(u64)` → `Vec<Address>`

| Property | Value |
|----------|-------|
| **Max size** | `MAX_REVIEWS_PER_PROJECT` = **500** |
| **Enforced on** | `add_review` / `submit_review` |
| **Error** | `MaxProjectsExceeded` (30) |
| **Read API** | `list_reviews(project_id, start, limit)` |
| **Pagination** | **Required** for large projects — use `list_reviews` with `start`/`limit` |
| **Notes** | One entry per unique reviewer; duplicates rejected with `DuplicateReview` |

### User reviews — `StorageKey::UserReviews(Address)` → `Vec<u64>`

| Property | Value |
|----------|-------|
| **Max size** | `MAX_REVIEWS_PER_USER` = **200** |
| **Enforced on** | `add_review` / `submit_review` |
| **Error** | `MaxProjectsExceeded` (30) |
| **Read API** | No dedicated paginated list — use events or per-project `get_review` |
| **Notes** | One entry per project the user has reviewed |

## Related bounded indexes (live)

| Index | Max size | Constant | Enforced on |
|-------|----------|----------|-------------|
| `StorageKey::CollectionList` | 100 | `MAX_COLLECTIONS` | `create_collection` |
| `StorageKey::CollectionProjectIds(id)` | 500 | `MAX_PROJECTS_PER_COLLECTION` | `add_project_to_collection` |

> **Note:** Soroban limits the number of contract error variants. Review and collection index caps reuse `MaxProjectsExceeded` (30), the same error returned when an owner exceeds `MAX_PROJECTS_PER_USER`.

## Other live Vec-backed indexes

These are written in production code and belong to the live scheme, but do not currently share the same hard `MAX_*` caps as the primary indexes above. Treat them as part of the index surface for exporters/indexers:

| Key | Value | Notes |
|-----|-------|-------|
| `StorageKey::CategoryProjects(String)` | `Vec<u64>` | Projects per category |
| `StorageKey::FeaturedProjects` | `Vec<u64>` | Featured project list |
| `StorageKey::AdminList` | `Vec<Address>` | Admin addresses |
| `StorageKey::ProjectVerificationHistory(u64)` | `Vec<u64>` | Verification request IDs |
| `StorageKey::ProjectTags(u64)` | `Vec<String>` | Project tags |
| `StorageKey::ProjectSocialLinks(u64)` | tag/link vec | Project social links |
| `StorageKey::ProjectMaintainers(u64)` | `Vec<Address>` | Maintainers |
| `StorageKey::ProjectLinkedProjects(u64)` | `Vec<u64>` | Linked projects |
| `StorageKey::ProjectReports(u64)` | report vec | Project reports |
| `ExtensionKey::ProjectClaimRequests(u64)` | `Vec<u64>` | Ownership claim request IDs |
| `ExtensionKey::ProjectDependencyKeys(u64)` | dependency keys | Dependency index |
| `ExtensionKey::ProjectDuplicateDisputes(u64)` | `Vec<u64>` | Dispute IDs |
| `ExtensionKey::ProjectFollowers(u64)` | followers | Subscription followers |
| `ExtensionKey::UserSubscriptions(Address)` | subscriptions | User follows |
| `ExtensionKey::UserBookmarks(Address)` | `Vec<u64>` | Bookmarked projects |
| `ExtensionKey::ProjectEndorsements(u64)` | endorsers | Endorsement list |
| `ExtensionKey::TimelockActionIds` | action IDs | Timelock queue |
| `ExtensionKey::AdminProposalIds` | proposal IDs | Admin governance |
| `ExtensionKey::ReservedNames` | `Vec<String>` | Reserved project names |

Lookup / singleton keys (not Vec indexes) such as `Project`, `ProjectBySlug`, `FeeConfig`, `FeePaidForProject`, claim records, etc. live in the same enums but are outside the scope of this size-strategy doc. See `docs/STORAGE_SCHEMA.md` for project field layout.

## Dead scaffolding (not part of the live scheme)

Cross-check against `storage_keys.rs` / `types.rs` found keys and types that are **defined but never read or written** by live contract paths. They must **not** be treated as indexes, documented as active storage, or relied on by indexers:

| Item | Location | Status |
|------|----------|--------|
| `DataKey` enum | `types.rs` | Legacy duplicate of `StorageKey` / partial keys; unused |
| `ExtensionKey::FeeRefundRecord(u64)` | `storage_keys.rs` | Defined only; no read/write path |
| `ExtensionKey::FeeConfigHistoryCount` | `storage_keys.rs` | Defined only; no read/write path |
| `ExtensionKey::FeeConfigHistoryEntry(u32)` | `storage_keys.rs` | Defined only; no read/write path |
| `FeeRefundRecord` struct | `types.rs` | Dead type paired with unused key |
| `FeeConfigHistoryEntry` struct | `types.rs` | Dead type paired with unused keys |
| `StorageKey::NextProjectId` | `storage_keys.rs` | Unused (counter is `ProjectCount`) |
| `StorageKey::ProjectLaunchTimestamp(u64)` | `storage_keys.rs` | Unused |

Do not add these to export guides or assume they exist on-chain.

## Future indexes

New `Vec`-based indexes should follow this pattern:

1. **Define a `MAX_*` constant** in `constants.rs` with a short rationale comment.
2. **Check length before `push_back`** on every write path (including admin/transfer side effects).
3. **Return a typed `ContractError`** when the cap is reached.
4. **Expose paginated reads** when the max can exceed `MAX_PAGE_LIMIT`.
5. **Add a boundary test** that succeeds at `MAX` and fails at `MAX + 1`.
6. **Add the variant to `StorageKey` or `ExtensionKey`** and update this document — never document dead scaffolding as live.

## Index cleanup

Deletes and moderation actions rebuild indexes by filtering entries (e.g. `delete_review`, `admin_delete_review`). Caps apply to live index length after cleanup, not historical high-water marks stored elsewhere.

## Integration checklist

- Use `get_owner_project_count` before batch registrations to pre-check capacity.
- Page `list_reviews` — never assume all reviewers fit in one call.
- Handle `MaxProjectsExceeded` on owner-project and review index writes in client error handling.
- For full-chain sync, combine events with paginated list endpoints (see `DATA_EXPORT_GUIDE.md`).
- Ignore dead scaffolding listed above when building indexers or migrations.
