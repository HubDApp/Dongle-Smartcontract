# Storage Schema

## Project Entry

A project is stored under the key `Project(u64)` and contains the following fields:

| Field | Type | Description | Optional |
|-------|------|-------------|----------|
| `id` | `u64` | Project ID (assigned on registration) | No |
| `owner` | `Address` | The project owner | No |
| `name` | `String` | Project name (≤ `MAX_NAME_LEN` = 50 bytes) | No |
| `slug` | `String` | URL-friendly identifier, unique (≤ `MAX_SLUG_LEN` = 64 bytes) | No |
| `description` | `String` | Project description (≤ `MAX_DESCRIPTION_LEN` = 2048 bytes) | No |
| `category` | `String` | Project category, e.g. "DeFi", "NFT" (≤ `MAX_CATEGORY_LEN` = 64 bytes) | No |
| `website` | `String` | Project website URL (≤ `MAX_WEBSITE_LEN` = 256 bytes) | Yes |
| `license` | `String` | License identifier / note (≤ `MAX_LICENSE_LEN` = 64 bytes) | Yes |
| `logo_cid` | `String` | IPFS CID for project logo (`MIN_CID_LEN`..=`MAX_CID_LEN`) | Yes |
| `metadata_cid` | `String` | IPFS CID for additional metadata (`MIN_CID_LEN`..=`MAX_CID_LEN`) | Yes |
| `verification_status` | `VerificationStatus` | Current verification state | No |
| `current_verification_id` | `u64` | Active verification record ID | Yes |
| `archived` | `bool` | Whether the project is archived | No (default `false`) |
| `claimable` | `bool` | Whether an ownership claim can be submitted | No (default `false`) |
| `lifecycle_status` | `ProjectLifecycleStatus` | Active / Deprecated / Abandoned / … | No |
| `tags` | `Vec<String>` | Associated tags (≤ `MAX_TAGS_PER_PROJECT` = 10, each ≤ `MAX_TAG_LENGTH` = 32 bytes) | Yes |
| `social_links` | `Map<String, String>` | Platform → URL (≤ `MAX_SOCIAL_LINKS` = 10; platform ≤ 32 bytes, URL ≤ 256 bytes) | Yes |
| `launch_timestamp` | `u64` | UNIX timestamp of project launch | Yes |
| `maintainers` | `Vec<Address>` | Maintainer addresses (defaults to empty list) | Yes |
| `bounty_url` | `String` | URL to bug bounty / disclosure policy (≤ `MAX_WEBSITE_LEN`) | Yes |
| `repository_url` | `String` | Source repository URL (≤ `MAX_WEBSITE_LEN`) | Yes |
| `security_contact` | `String` | Published security contact (≤ `MAX_SECURITY_CONTACT_LEN` = 256 bytes) | Yes |
| `security_contact_proof_cid` | `String` | CID proving control of the security contact (`MIN_CID_LEN`..=`MAX_CID_LEN`) | Yes |
| `created_at` | `u64` | Timestamp of project creation | No |
| `updated_at` | `u64` | Timestamp of last update | No |

## Review Entry

Stored under `Review(u64, Address)` (project ID + reviewer):

| Field | Type | Description | Optional |
|-------|------|-------------|----------|
| `project_id` | `u64` | Reviewed project | No |
| `reviewer` | `Address` | Review author | No |
| `rating` | `u32` | Star rating, `RATING_MIN`..=`RATING_MAX` (1–5) | No |
| `content_cid` | `String` | IPFS CID for review body (`MIN_CID_LEN`..=`MAX_CID_LEN`) | Yes |
| `owner_response` | `String` | Owner's public response (≤ `MAX_CID_LEN`) | Yes |
| `created_at` / `updated_at` / `last_updated_at` | `u64` | Timestamps | No |
| `hidden` | `bool` | Hidden by moderation | No |
| `report_count` | `u32` | Number of reports filed | No |

Reviewer edit history is a ring buffer capped at `MAX_REVIEW_REVISIONS` = 50.

## Collection Entry

Stored under `Collection(u64)`:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `u64` | Collection ID |
| `name` | `String` | Display title (≤ `MAX_COLLECTION_NAME_LEN` = 100 bytes) |
| `description` | `String` | Blurb (≤ `MAX_COLLECTION_DESCRIPTION_LEN` = 500 bytes) |
| `created_at` / `updated_at` | `u64` | Timestamps |

Membership is a `Vec<u64>` capped at `MAX_PROJECTS_PER_COLLECTION` = 500; the
global number of collections is capped at `MAX_COLLECTIONS` = 100.

---

## Field Length & Collection Limits

All limits live in
[`dongle-smartcontract/src/constants.rs`](../dongle-smartcontract/src/constants.rs)
and are enforced on **every** write path (`register_project`, `update_project`,
`add_review`, `create_collection`, …). String limits are **byte** lengths, not
character counts. Boundary behaviour (`N` accepted, `N + 1` rejected) is covered
by [`src/tests/field_limits.rs`](../dongle-smartcontract/src/tests/field_limits.rs).

### String fields

| Constant | Value (bytes) | Applies to | Error on violation | Rationale |
|----------|---------------|------------|--------------------|-----------|
| `MIN_STRING_LEN` | 1 | name, description, category (non-empty, non-whitespace) | `InvalidProjectName` / `InvalidProjectData` / `InvalidInput` | An empty field carries no information and breaks slug/URL/list rendering. |
| `MAX_NAME_LEN` | 50 | `Project.name` | `InvalidProjectName` | Renders in one list row; a slug is derived from it. Validation copies into a `[u8; 128]` stack buffer. |
| `MAX_SLUG_LEN` | 64 | `Project.slug` | `InvalidProjectSlug` | URL path segment. Slightly larger than the name so a max-length name still slugifies without truncation. |
| `MAX_DESCRIPTION_LEN` | 2048 | `Project.description` | `InvalidProjectData` | The only long-form field. ~30× below the Soroban ledger entry ceiling so the full project entry still serialises. Validation buffer is `[u8; 2048]`. |
| `MAX_CATEGORY_LEN` | 64 | `Project.category` | `InvalidInput` | Categories are a small controlled vocabulary; the limit only guards against abuse of the free-text field. |
| `MAX_WEBSITE_LEN` | 256 | `website`, `bounty_url`, `repository_url`, social link URLs | `InvalidInput` | Covers real project landing-page / profile URLs while bounding the `[u8; 256]` validation buffer. |
| `MAX_LICENSE_LEN` | 64 | `Project.license` | `InvalidInput` | Fits SPDX identifiers (`Apache-2.0`, `GPL-3.0-or-later`) and short license notes. |
| `MAX_SECURITY_CONTACT_LEN` | 256 | `Project.security_contact` | `InvalidInput` | Covers a `mailto:`, an HTTPS disclosure page, or a `security.txt`-style line. |
| `MAX_CID_LEN` | 128 | every CID field (logo, metadata, review content, evidence, changelog, proof) | `InvalidCid` | CIDv1 base32 is ~59 chars; 128 leaves headroom for longer multibase encodings without allowing arbitrary blobs. |
| `MIN_CID_LEN` | 46 | every CID field | `InvalidCid` | A CIDv0 is exactly 46 base58 chars; shorter cannot be valid. |
| `MAX_TAG_LENGTH` | 32 | each entry in `Project.tags` | `InvalidTags` | Tags are single words / short slugs (`[A-Za-z0-9_-]`). |
| `MAX_SOCIAL_LINK_PLATFORM_LEN` | 32 | social link platform key | `InvalidInput` | Platform labels (`twitter`, `discord`, `github`) are short. |
| `MAX_SOCIAL_LINK_URL_LEN` | 256 | social link URL value | `InvalidInput` | Same shape as `MAX_WEBSITE_LEN`. |
| `MAX_COLLECTION_NAME_LEN` | 100 | `Collection.name` | `InvalidInput` | Curator-authored display titles, allowed to be more descriptive than a project name. |
| `MAX_COLLECTION_DESCRIPTION_LEN` | 500 | `Collection.description` | `InvalidInput` | A short blurb, not long-form prose. |

### Count / cardinality fields

| Constant | Value | Applies to | Error on violation | Rationale |
|----------|-------|------------|--------------------|-----------|
| `MAX_PROJECTS_PER_USER` | 50 | `OwnerProjects(Address)` index | `MaxProjectsExceeded` | The index is a `Vec<u64>` rewritten on every register/transfer/archive; 50 keeps the read-modify-write cheap and blocks Sybil index bloat. |
| `MAX_TAGS_PER_PROJECT` | 10 | `Project.tags` | `InvalidTags` | Stored inline and scanned linearly on every tag-index update. |
| `MAX_SOCIAL_LINKS` | 10 | `Project.social_links` | `InvalidInput` | Same bound-on-write reasoning as tags; covers every mainstream platform. |
| `MAX_REVIEWS_PER_PROJECT` | 500 | `ProjectReviews(u64)` index | `MaxProjectsExceeded` | Vec index capped on write; pagination via `list_reviews`. |
| `MAX_REVIEWS_PER_USER` | 200 | `UserReviews(Address)` index | `MaxProjectsExceeded` | Vec index capped on write. |
| `MAX_REVIEW_REVISIONS` | 50 | per-review edit history | oldest dropped (no error) | Bounded ring buffer so an actively-edited review cannot grow without limit. |
| `MAX_COLLECTIONS` | 100 | global collection count | `MaxProjectsExceeded` | Curated admin-scale feature; keeps `list_collections` pagination bounded. |
| `MAX_PROJECTS_PER_COLLECTION` | 500 | `Collection` membership `Vec<u64>` | `CollectionFull` | 500 × 8 bytes = 4 KiB, well within one ledger entry. |
| `MAX_PAGE_LIMIT` | 100 | every paginated read | clamped (no error) | Caps result-set size / CPU per query. |
| `MAX_TTL_BATCH_SIZE` | 100 | `extend_projects_ttl` / `extend_reviews_ttl` | `InvalidInput` | Caps per-call storage writes. |
| `MAX_ADMIN_ACTION_LOG_PAGE` | 100 | `list_admin_actions` | clamped | As `MAX_PAGE_LIMIT`. |

### Why these ceilings

1. **Soroban ledger entry size.** A persistent entry must stay well under the
   ~64 KiB XDR limit. A project entry aggregates ~20 fields plus `Vec`/`Map`
   indexes, so each field is kept to the smallest size that fits its purpose;
   `MAX_DESCRIPTION_LEN` (2048) is the single largest contributor and is still
   ~30× under the ceiling.
2. **CPU / instruction metering.** Character-set validation in
   [`utils.rs`](../dongle-smartcontract/src/utils.rs) copies each string into a
   fixed-size stack buffer sized from these constants. Smaller limits → smaller
   stack frames and cheaper metered execution.
3. **Index write amplification.** `Vec`-based indexes (`OwnerProjects`,
   `ProjectReviews`, collection membership) are fully rewritten on each mutation.
   The cardinality caps bound that O(n) cost. See
   [`STORAGE_INDEXES.md`](./STORAGE_INDEXES.md).
4. **Display / UX.** Names, slugs, categories and tags surface in lists and URLs
   and are intentionally short.

### Changing a limit

- Update the constant in `constants.rs` (keep the rationale comment current).
- Update the tables above.
- Update / add the boundary case in `src/tests/field_limits.rs`.
- **Never lower** a string limit on a deployed contract without a migration
  plan — existing entries at the old length remain readable but can no longer
  be re-saved through `update_*`.

## Off-Chain JSON Schemas

The following JSON schema and example files are co-located here in `docs/`:

| File | Purpose |
|------|---------|
| [`project-metadata.schema.json`](./project-metadata.schema.json) | Project metadata JSON schema |
| [`project-metadata.example.json`](./project-metadata.example.json) | Example valid project metadata document |
| [`review-cid.schema.json`](./review-cid.schema.json) | Review content JSON schema |
| [`review-cid.example.json`](./review-cid.example.json) | Example valid review document |

## Validation Rules

- `bounty_url`: If provided, must be a valid HTTP/HTTPS URL (starts with `http://` or `https://`).
- `bounty_cid`: If provided, must be a valid IPFS CID (v0 starting with `Qm` and 46 characters, or v1 starting with `b` and at least 40 characters).
- All string fields: byte length within the limits in
  [Field Length & Collection Limits](#field-length--collection-limits); rejected
  otherwise with the listed error.

## Indexes

| Key | Value | Description |
|-----|-------|-------------|
| `OwnerProjects(Address)` | `Vec<u64>` | List of project IDs owned by an address (≤ `MAX_PROJECTS_PER_USER`) |
| `ProjectCount` | `u64` | Total number of registered projects |
| `ProjectSlug(String)` | `u64` | Slug-to-ID lookup |
| `FeaturedProjects` | `Vec<u64>` | List of featured project IDs |
| `ProjectReviews(u64)` | `Vec<Address>` | Reviewers of a project (≤ `MAX_REVIEWS_PER_PROJECT`) |
| `UserReviews(Address)` | `Vec<u64>` | Projects reviewed by an address (≤ `MAX_REVIEWS_PER_USER`) |

See [`STORAGE_INDEXES.md`](./STORAGE_INDEXES.md) for the full index catalog.
