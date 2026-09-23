# Design Document: Review Evidence Links

## Overview

This feature extends the Dongle-Smartcontract review system to let reviewers attach up to 5 external URLs as supporting evidence or documentation alongside their review. The contract stores, validates, and exposes these links on-chain. Dead-link detection, reachability checking, and metadata extraction are intentionally off-chain concerns delegated to indexers and frontend clients — Soroban smart contracts have no outbound networking capability.

The core changes are:
- A new `EvidenceLink` type with a URL string and an admin-settable `is_dead` flag.
- A new storage key for evidence links, residing in a new `ExtensionKey2` enum (since `ExtensionKey` is at its 50-variant Soroban cap).
- Modified `add_review` / `submit_review` / `update_review` / `delete_review` / `admin_delete_review` signatures to carry an `Option<Vec<EvidenceLink>>` parameter.
- A new `get_review_evidence_links` read entry point and a `mark_evidence_link_dead` admin entry point.
- `ReviewEventData` updated to carry the current `evidence_links` snapshot.
- `ContractLimits` updated to surface `max_evidence_links_per_review`.

---

## Architecture

The feature is implemented entirely within `dongle-smartcontract/src/`. No new modules are introduced; changes are distributed across four layers:

```
┌────────────────────────────────────────────────┐
│  lib.rs  (contract entry points)               │
│  add_review / submit_review / update_review … │
│  get_review_evidence_links                     │
│  mark_evidence_link_dead                       │
└───────────────────┬────────────────────────────┘
                    │ delegates to
┌───────────────────▼────────────────────────────┐
│  review_registry/storage.rs  (business logic) │
│  add_review_impl / update_review_impl          │
│  store/load/delete evidence links              │
│  mark_evidence_link_dead_impl                  │
└───────────────────┬────────────────────────────┘
                    │ calls
┌───────────────────▼────────────────────────────┐
│  review_registry/validation.rs  (validation)  │
│  validate_evidence_links(links)               │
│  validate_evidence_link_url(url)              │
└───────────────────┬────────────────────────────┘
                    │ reads
┌───────────────────▼────────────────────────────┐
│  constants.rs  MAX_EVIDENCE_LINKS_PER_REVIEW   │
│  MAX_EVIDENCE_LINK_URL_LEN (512)               │
└────────────────────────────────────────────────┘
```

**Storage key placement.** `ExtensionKey` currently has exactly 50 variants — the Soroban `#[contracttype]` hard cap. Both new storage keys (`ReviewEvidenceLinks` and `ReviewEvidenceLinkDead`) will live in a new `ExtensionKey2` enum following the exact pattern documented in `storage_keys.rs`.

---

## Components and Interfaces

### New types (`types.rs`)

```rust
/// A single URL attached to a review as supporting evidence.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceLink {
    /// The URL string (http:// or https://).
    pub url: String,
    /// Admin-settable dead-link flag. False by default.
    pub is_dead: bool,
}
```

`Review` gains a new field:

```rust
pub struct Review {
    // … existing fields unchanged …
    /// Optional list of attached evidence links (max MAX_EVIDENCE_LINKS_PER_REVIEW).
    pub evidence_links: Vec<EvidenceLink>,
}
```

`ReviewEventData` gains the same field so every review event carries a snapshot:

```rust
pub struct ReviewEventData {
    // … existing fields unchanged …
    pub evidence_links: Vec<EvidenceLink>,
}
```

`ContractLimits` gains:

```rust
pub struct ContractLimits {
    // … existing fields unchanged …
    /// Maximum evidence links attachable to a single review.
    pub max_evidence_links_per_review: u32,
}
```

### New storage keys (`storage_keys.rs`)

```rust
/// Third overflow enum, introduced because ExtensionKey reached 50 variants.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtensionKey2 {
    /// Evidence links for a review, keyed by (project_id, reviewer).
    ReviewEvidenceLinks(u64, Address),
}
```

`ReviewEvidenceLinkDead` is **not** a separate storage key. The `is_dead` flag is stored inline inside each `EvidenceLink` struct within the `Vec<EvidenceLink>` under `ReviewEvidenceLinks`. The `mark_evidence_link_dead` entry point reads the list, flips the flag on the specified index, and writes the list back. This avoids a second key and keeps the data co-located.

### New constants (`constants.rs`)

```rust
/// Maximum number of evidence links that may be attached to a single review.
pub const MAX_EVIDENCE_LINKS_PER_REVIEW: u32 = 5;

/// Maximum byte length of a single evidence link URL.
pub const MAX_EVIDENCE_LINK_URL_LEN: usize = 512;
```

### Validation (`review_registry/validation.rs`)

```rust
impl ReviewValidation {
    /// Validate the full list of evidence links for a review submission or update.
    pub fn validate_evidence_links(
        links: &Vec<EvidenceLink>,
    ) -> Result<(), ContractError> {
        if links.len() as u32 > MAX_EVIDENCE_LINKS_PER_REVIEW {
            return Err(ContractError::TooManyEvidenceLinks);
        }
        for i in 0..links.len() {
            if let Some(link) = links.get(i) {
                Self::validate_evidence_link_url(&link.url)?;
            }
        }
        Ok(())
    }

    /// Validate a single evidence link URL.
    ///
    /// Rules:
    /// - Non-empty.
    /// - Starts with `https://` or `http://`.
    /// - At most MAX_EVIDENCE_LINK_URL_LEN bytes.
    pub fn validate_evidence_link_url(url: &String) -> Result<(), ContractError> {
        let len = url.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidEvidenceLink);
        }
        if len > MAX_EVIDENCE_LINK_URL_LEN {
            return Err(ContractError::EvidenceLinkTooLong);
        }
        // Copy into a stack buffer for prefix check
        let mut buf = [0u8; MAX_EVIDENCE_LINK_URL_LEN];
        url.copy_into_slice(&mut buf[..len]);
        if !buf[..len].starts_with(b"https://")
            && !buf[..len].starts_with(b"http://")
        {
            return Err(ContractError::InvalidEvidenceLink);
        }
        Ok(())
    }
}
```

Note: The existing `validate_website` on `Utils` accepts only `https://`. Evidence links intentionally accept both `http://` and `https://` (reviewer-attached proof may live on legacy HTTP hosts), so a dedicated validator is warranted rather than re-using `validate_website`.

### Entry point changes (`lib.rs`)

All existing review entry-point signatures gain a trailing `evidence_links: Option<Vec<EvidenceLink>>` parameter:

| Entry point | Behaviour of `None` | Behaviour of `Some(links)` |
|---|---|---|
| `add_review` | Store empty links | Validate and store `links` |
| `submit_review` | Store empty links | Validate and store `links` |
| `update_review` | Leave existing links unchanged | Validate and replace links |

`Some(Vec::new())` (explicitly empty) on an update clears all links.

New entry points added:

```rust
/// Return the evidence links for a review. Returns empty Vec for missing reviews.
pub fn get_review_evidence_links(
    env: Env,
    project_id: u64,
    reviewer: Address,
) -> Vec<EvidenceLink>

/// Admin-only: mark a specific evidence link index as dead.
pub fn mark_evidence_link_dead(
    env: Env,
    admin: Address,
    project_id: u64,
    reviewer: Address,
    link_index: u32,
) -> Result<(), ContractError>
```

### Storage manager (`storage_manager.rs`)

`extend_review_ttl` is extended to also bump the `ExtensionKey2::ReviewEvidenceLinks` entry:

```rust
pub fn extend_review_ttl(env: &Env, project_id: u64, reviewer: &Address) {
    // … existing bump of StorageKey::Review …
    Self::extend_if_exists(
        env,
        &ExtensionKey2::ReviewEvidenceLinks(project_id, reviewer.clone()),
        LEDGER_THRESHOLD_REVIEW,
        LEDGER_BUMP_REVIEW,
    );
}
```

### New error variants (`errors.rs`)

```rust
/// Too many evidence links supplied (max MAX_EVIDENCE_LINKS_PER_REVIEW).
TooManyEvidenceLinks = 82,
/// Evidence link URL is empty or has an invalid scheme.
InvalidEvidenceLink = 83,
/// Evidence link URL exceeds MAX_EVIDENCE_LINK_URL_LEN bytes.
EvidenceLinkTooLong = 84,
```

---

## Data Models

### `EvidenceLink`

| Field | Type | Description |
|---|---|---|
| `url` | `String` | The evidence URL (http:// or https://), max 512 bytes |
| `is_dead` | `bool` | Set to `true` by an admin to flag a dead link |

### `Review` (updated)

The `evidence_links` field is appended after all existing fields to preserve ABI forward-compatibility (callers that do not read new fields are unaffected).

### Storage layout

| Key | Type | Description |
|---|---|---|
| `ExtensionKey2::ReviewEvidenceLinks(project_id, reviewer)` | `Vec<EvidenceLink>` | Evidence links for a (project, reviewer) pair |

Evidence links are stored **separately** from the `Review` struct. This keeps the core `Review` entry small and avoids re-serialising the full review record when only links change. `get_review` assembles the combined view by reading both keys.

### TTL

Evidence link entries share the review TTL tier: `LEDGER_THRESHOLD_REVIEW` / `LEDGER_BUMP_REVIEW` (~60 days). `StorageManager::extend_review_ttl` is updated to extend both entries together.

---

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Evidence links round-trip on submission

*For any* valid `(project_id, reviewer, rating, evidence_links)` tuple where `evidence_links` satisfies all format and count constraints, submitting a review and then calling `get_review_evidence_links` SHALL return a list equal to the submitted list.

**Validates: Requirements 1.1, 4.1**

---

### Property 2: Over-limit submission is always rejected

*For any* evidence link list whose length strictly exceeds `MAX_EVIDENCE_LINKS_PER_REVIEW`, calling `add_review` or `submit_review` SHALL return `ContractError::TooManyEvidenceLinks` and leave the review list unchanged.

**Validates: Requirements 1.3, 9.2**

---

### Property 3: URL scheme and length validation

*For any* URL string, the link validator SHALL accept it if and only if it starts with `https://` or `http://` AND its byte length is in `[1, MAX_EVIDENCE_LINK_URL_LEN]`. Any URL that violates either condition SHALL be rejected.

**Validates: Requirements 2.1, 2.2, 2.3**

---

### Property 4: Update replaces links

*For any* existing review with stored evidence links and any valid replacement list, calling `update_review` with `Some(replacement)` SHALL result in `get_review_evidence_links` returning `replacement` and not the original list.

**Validates: Requirements 3.1**

---

### Property 5: Update with None preserves links

*For any* existing review with stored evidence links, calling `update_review` with `None` for evidence links SHALL leave the stored evidence links unchanged.

**Validates: Requirements 3.2**

---

### Property 6: Update with Some([]) clears links

*For any* existing review with any stored evidence links, calling `update_review` with `Some(Vec::new())` SHALL result in `get_review_evidence_links` returning an empty list.

**Validates: Requirements 3.3**

---

### Property 7: Deletion removes evidence links

*For any* review (deleted either by the reviewer via `delete_review` or by an admin via `admin_delete_review`) that had stored evidence links, a subsequent call to `get_review_evidence_links` for the same `(project_id, reviewer)` pair SHALL return an empty list.

**Validates: Requirements 5.1, 5.2, 5.3**

---

### Property 8: Review events carry evidence links snapshot

*For any* review action (Submitted, Updated, or Deleted), the emitted `ReviewEventData` SHALL contain an `evidence_links` field equal to the evidence links that were current at the time the action was executed.

**Validates: Requirements 6.1, 6.2, 6.3**

---

### Property 9: mark_evidence_link_dead requires admin

*For any* address that is not a registered admin, calling `mark_evidence_link_dead` SHALL return `ContractError::AdminOnly` and SHALL NOT modify any stored evidence link.

**Validates: Requirements 8.4**

---

### Property 10: mark_evidence_link_dead sets flag precisely

*For any* review with `N` evidence links and any valid index `i` in `[0, N)`, calling `mark_evidence_link_dead(i)` SHALL result in `links[i].is_dead == true` while all other links remain unchanged, and calling it a second time on the same index SHALL be idempotent.

**Validates: Requirements 8.3**

---

## Error Handling

| Scenario | Error |
|---|---|
| List length > `MAX_EVIDENCE_LINKS_PER_REVIEW` | `TooManyEvidenceLinks` (82) |
| URL is empty string | `InvalidEvidenceLink` (83) |
| URL lacks `http://` or `https://` prefix | `InvalidEvidenceLink` (83) |
| URL byte length > `MAX_EVIDENCE_LINK_URL_LEN` | `EvidenceLinkTooLong` (84) |
| `mark_evidence_link_dead` by non-admin | `AdminOnly` (10) |
| `mark_evidence_link_dead` — review not found | `ReviewNotFound` (5) |
| `mark_evidence_link_dead` — index out of bounds | `InvalidInput` (36) |

All validation errors from evidence links propagate through the same `Result<(), ContractError>` return path as existing review validation errors. The calling entry point fails atomically — no partial state is written if any link fails validation.

---

## Testing Strategy

### Unit / example-based tests

Specific scenarios that concrete examples validate best:

- Happy-path submission with zero links (backward-compatible call).
- Happy-path submission with exactly `MAX_EVIDENCE_LINKS_PER_REVIEW` links.
- `update_review` with `None` leaves links unchanged.
- `update_review` with `Some([])` clears links.
- `get_review_evidence_links` on a non-existent review returns empty `Vec`.
- `mark_evidence_link_dead` by admin on valid index succeeds.
- `mark_evidence_link_dead` on second call is idempotent.
- `get_config` returns `ContractLimits` with `max_evidence_links_per_review == 5`.
- `extend_review_ttl` touches the `ReviewEvidenceLinks` storage entry.
- `admin_delete_review` removes evidence links.

### Property-based tests (proptest)

Each property listed in the Correctness Properties section maps to one property-based test using [`proptest`](https://proptest-rs.github.io/proptest/). Tests run a minimum of 100 iterations.

Tag format: `// Feature: review-evidence-links, Property N: <property_text>`

| Property | Generator strategy | Assertion |
|---|---|---|
| P1 – round-trip | Random (project_id, reviewer, valid links list [0..5]) | `get_review_evidence_links == submitted` |
| P2 – over-limit rejected | Random links list of length 6..=20 | Returns `TooManyEvidenceLinks`, review list size unchanged |
| P3 – URL scheme/length | Random strings with valid/invalid prefix and length | Accept iff prefix ∈ {http://, https://} AND len ≤ 512 |
| P4 – update replaces | Random initial list + random replacement list (both valid) | `get_review_evidence_links == replacement` |
| P5 – None preserves | Random initial links | Links unchanged after update with `None` |
| P6 – Some([]) clears | Random initial links | Links empty after update with `Some([])` |
| P7 – deletion cleans up | Random links, random delete actor (owner or admin) | `get_review_evidence_links` returns empty |
| P8 – event snapshot | Random links, random action | Event `evidence_links` field equals action-time state |
| P9 – admin-only | Random non-admin addresses | Returns `AdminOnly` |
| P10 – dead-flag precision | Random valid links list, random index in range | Targeted link `is_dead == true`, all others unchanged; second call idempotent |

### Integration points

The existing review test helpers in `dongle-smartcontract/src/tests/` will be updated to pass `None` for `evidence_links` where they call `add_review`, `submit_review`, or `update_review`, maintaining full backward compatibility of the test suite. New test files will be added:

- `src/tests/review_evidence_links.rs` — unit + property tests for the evidence link feature.
