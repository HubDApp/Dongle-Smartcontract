# Issue #819: Allow Collections to be Public or Private

## Overview
- **Issue**: [#819 - Allow collections to be public or private](https://github.com/HubDApp/Dongle-Smartcontract/issues/819)
- **Repository**: [HubDApp/Dongle-Smartcontract](https://github.com/HubDApp/Dongle-Smartcontract)
- **Target Branch**: `main`
- **Feature Branch**: `feat/public-private-collections`

## Description
Collections can be shared publicly or kept private to the creator/owner. This implementation adds visibility control, discoverability filtering, owner-only authorization checks, and secure share link / capability token generation for selective sharing.

## Acceptance Criteria
1. **Toggle visibility per collection**: Owners can toggle collections between Public and Private, and set visibility at creation time.
2. **Public collections searchable/discoverable**: Only public collections are indexed and returned via general browse/search queries (`list_collections`).
3. **Private collections only for owner**: Private collections are inaccessible and hidden from unauthorized callers; only the collection owner (or contract admin) can retrieve or view projects in their private collection.
4. **Share link generation for collections**: Owners can generate cryptographic share links/tokens to selectively grant read access to private collections.

---

## Architecture & Data Design

### 1. Storage Key Namespace (`CollectionVisibilityKey`)
To respect the strict Soroban 50-variant limit on `StorageKey` and `ExtensionKey`, all visibility, ownership, and share-link state is namespaced under a new dedicated enum in `dongle-smartcontract/src/storage_keys.rs`:

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollectionVisibilityKey {
    /// Owner address of a collection: collection_id -> Address
    CollectionOwner(u64),
    /// Visibility state: collection_id -> bool (true = public, false = private)
    CollectionIsPublic(u64),
    /// Active share token hash: collection_id -> String
    CollectionShareToken(u64),
    /// Index of collection IDs owned by a user: Address -> Vec<u64>
    UserCollections(Address),
    /// Index of public collection IDs for discovery: Vec<u64>
    PublicCollectionList,
}
```

### 2. Collection Type (`types.rs`)
The `Collection` struct tracks the collection's owner and visibility status, defaulting `is_public` to `true` for backwards compatibility:

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collection {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub is_public: bool,
    pub created_at: u64,
    pub updated_at: u64,
}
```

### 3. Events (`events.rs`)
- `CollectionVisibilityToggledEvent { collection_id: u64, is_public: bool, caller: Address }`
- `CollectionShareLinkGeneratedEvent { collection_id: u64, caller: Address }`
- `CollectionShareLinkRevokedEvent { collection_id: u64, caller: Address }`

### 4. Errors (`errors.rs`)
- `CollectionPrivate = 120`: Attempted unauthorized access to a private collection
- `NotCollectionOwner = 121`: Caller is neither the owner of the collection nor an admin
- `InvalidShareToken = 122`: The supplied share token does not match the active link
- `ShareTokenNotFound = 123`: No active share link exists for this collection

---

## Contract Interface (`lib.rs`)

```rust
// ── Visibility & Ownership ───────────────────────────────────────────

/// Toggle collection visibility between public and private (owner or admin only).
pub fn toggle_collection_visibility(
    env: Env,
    caller: Address,
    collection_id: u64,
) -> Result<bool, ContractError>;

/// Set explicit collection visibility (owner or admin only).
pub fn set_collection_visibility(
    env: Env,
    caller: Address,
    collection_id: u64,
    is_public: bool,
) -> Result<(), ContractError>;

/// Retrieve a collection with caller verification (owner can see private collections).
pub fn get_collection_for_caller(
    env: Env,
    caller: Address,
    collection_id: u64,
) -> Result<Collection, ContractError>;

/// List all collections owned by a user (requires user auth).
pub fn list_user_collections(
    env: Env,
    owner: Address,
    start_index: u32,
    limit: u32,
) -> Result<Vec<Collection>, ContractError>;

// ── Share Links ──────────────────────────────────────────────────────

/// Generate a unique, cryptographically seeded share link for a collection.
pub fn generate_collection_share_link(
    env: Env,
    caller: Address,
    collection_id: u64,
) -> Result<String, ContractError>;

/// Access a private collection via a valid share token.
pub fn get_collection_by_share_token(
    env: Env,
    collection_id: u64,
    share_token: String,
) -> Result<Collection, ContractError>;

/// Revoke an active share link for a collection.
pub fn revoke_collection_share_link(
    env: Env,
    caller: Address,
    collection_id: u64,
) -> Result<(), ContractError>;
```

---

## Step-by-Step Implementation Phases

### Phase 1: Storage Keys, Types, Events & Errors
1. Add `CollectionVisibilityKey` to `dongle-smartcontract/src/storage_keys.rs`.
2. Update `Collection` struct in `dongle-smartcontract/src/types.rs` with `owner: Address` and `is_public: bool`.
3. Add `CollectionPrivate`, `NotCollectionOwner`, `InvalidShareToken`, and `ShareTokenNotFound` to `dongle-smartcontract/src/errors.rs`.
4. Add visibility and share link events in `dongle-smartcontract/src/events.rs`.

### Phase 2: Registry Logic (`collection_registry.rs`)
1. **Creation**:
   - Save `owner` and track in `CollectionVisibilityKey::UserCollections(owner)`.
   - If `is_public == true`, append ID to `PublicCollectionList`.
2. **Toggle Visibility (AC1)**:
   - Verify `caller == owner || is_admin(&caller)`.
   - Flip `collection.is_public`.
   - Update `PublicCollectionList` index (remove if private, push if public).
   - Emit `CollectionVisibilityToggledEvent`.
3. **Discoverability (AC2)**:
   - `list_collections(env, start_index, limit)` iterates over `PublicCollectionList` only.
4. **Privacy Enforcements (AC3)**:
   - Public getter `get_collection(env, id)` returns `None` for private collections.
   - `get_collection_for_caller(env, caller, id)` grants access to owner/admin, returns `Err(CollectionPrivate)` otherwise.
   - Guard `list_collection_projects` against unauthorized inspection of private collections.
5. **Share Links (AC4)**:
   - `generate_share_link`: Computes `token = sha256(collection_id, owner, ledger_sequence, timestamp)`.
   - Formats link: `https://dongle.hub/collections/{id}?key={token_hex}`.
   - Persists token under `CollectionVisibilityKey::CollectionShareToken(collection_id)`.
   - `get_collection_by_share_token`: Validates token and returns collection.
   - `revoke_share_link`: Removes token from storage.

### Phase 3: Contract Interface (`lib.rs`)
Expose all public methods through the Soroban contract entrypoints.

### Phase 4: Automated Testing (`tests/collection_visibility.rs`)
- `test_create_with_visibility_defaults_and_explicit`
- `test_toggle_collection_visibility`
- `test_non_owner_cannot_toggle_visibility`
- `test_public_collections_discoverable_via_list`
- `test_private_collection_hidden_from_public_list_and_getter`
- `test_private_collection_accessible_by_owner`
- `test_private_collection_denied_for_stranger`
- `test_generate_and_access_via_share_link`
- `test_revoke_share_link`
- `test_invalid_share_link_rejected`

### Phase 5: Documentation
- Update `docs/CONTRACT_INTERFACE.md`.
- Update `docs/STORAGE_SCHEMA.md`.
- Add release note in `CHANGELOG.md`.

---

## Status
- Branch: `feat/public-private-collections`
- Readiness: Ready for implementation
