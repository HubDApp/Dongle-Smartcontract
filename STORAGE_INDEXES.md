# Storage Index Size Strategy

On-chain indexes in Dongle are `Vec`-backed storage entries keyed by `StorageKey`. Each index grows on write (`push_back`) and is trimmed on delete. Without caps, a single user or project could accumulate unbounded entries and exhaust Soroban storage budgets.

This document defines **maximum expected index sizes**, **write-time enforcement**, and **read pagination** for index consumers (frontends and off-chain indexers).

Constants live in `dongle-smartcontract/src/constants.rs`.

## Pagination (read path)

All paginated list endpoints clamp `limit` to `MAX_PAGE_LIMIT` (**100**). When `limit` is `0` or greater than 100, the effective limit is 100.

| Parameter | Behavior |
|-----------|----------|
| `limit = 0` | Treated as `MAX_PAGE_LIMIT` (100) on most endpoints |
| `limit > 100` | Clamped to 100 |
| `start` / `start_id` / `start_index` | Zero-based offset into the index Vec |

Indexers should page through large indexes using these parameters rather than assuming a single call returns all entries.

## Primary indexes (this issue)

### Owner projects — `OwnerProjects(Address)` → `Vec<u64>`

| Property | Value |
|----------|-------|
| **Max size** | `MAX_PROJECTS_PER_USER` = **50** |
| **Enforced on** | `register_project`, `accept_transfer`, `approve_claim_request` |
| **Error** | `MaxProjectsExceeded` (30) |
| **Read API** | `get_projects_by_owner`, `get_owner_project_count` |
| **Pagination** | Not required — index is bounded at 50; full fetch is safe |
| **Notes** | Archived projects remain in the index; `get_projects_by_owner` filters them at read time |

### Project reviews — `ProjectReviews(u64)` → `Vec<Address>`

| Property | Value |
|----------|-------|
| **Max size** | `MAX_REVIEWS_PER_PROJECT` = **500** |
| **Enforced on** | `add_review` / `submit_review` |
| **Error** | `MaxProjectsExceeded` (30) |
| **Read API** | `list_reviews(project_id, start, limit)` |
| **Pagination** | **Required** for large projects — use `list_reviews` with `start`/`limit` |
| **Notes** | One entry per unique reviewer; duplicates rejected with `DuplicateReview` |

### User reviews — `UserReviews(Address)` → `Vec<u64>`

| Property | Value |
|----------|-------|
| **Max size** | `MAX_REVIEWS_PER_USER` = **200** |
| **Enforced on** | `add_review` / `submit_review` |
| **Error** | `MaxProjectsExceeded` (30) |
| **Read API** | No dedicated paginated list — use events or per-project `get_review` |
| **Notes** | One entry per project the user has reviewed |

## Related bounded indexes (existing)

| Index | Max size | Constant | Enforced on |
|-------|----------|----------|-------------|
| `CollectionList` | 100 | `MAX_COLLECTIONS` | `create_collection` |
| `CollectionProjectIds(id)` | 500 | `MAX_PROJECTS_PER_COLLECTION` | `add_project_to_collection` |

> **Note:** Soroban limits the number of contract error variants. Review and collection index caps reuse `MaxProjectsExceeded` (30), the same error returned when an owner exceeds `MAX_PROJECTS_PER_USER`.

## Future indexes

New `Vec`-based indexes should follow this pattern:

1. **Define a `MAX_*` constant** in `constants.rs` with a short rationale comment.
2. **Check length before `push_back`** on every write path (including admin/transfer side effects).
3. **Return a typed `ContractError`** when the cap is reached.
4. **Expose paginated reads** when the max can exceed `MAX_PAGE_LIMIT`.
5. **Add a boundary test** that succeeds at `MAX` and fails at `MAX + 1`.

## Index cleanup

Deletes and moderation actions rebuild indexes by filtering entries (e.g. `delete_review`, `admin_delete_review`). Caps apply to live index length after cleanup, not historical high-water marks stored elsewhere.

## Integration checklist

- Use `get_owner_project_count` before batch registrations to pre-check capacity.
- Page `list_reviews` — never assume all reviewers fit in one call.
- Handle `MaxProjectsExceeded` on owner-project and review index writes in client error handling.
- For full-chain sync, combine events with paginated list endpoints (see `DATA_EXPORT_GUIDE.md`).
