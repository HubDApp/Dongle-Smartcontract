# Dongle Smart Contract Interface Documentation

## Overview

This document provides comprehensive documentation of all public contract functions in the Dongle smart contract. Each function includes its purpose, parameters, return values, authorization requirements, and possible errors.

**Coverage**: all **198** `pub fn` entry points in [`lib.rs`](../dongle-smartcontract/src/lib.rs) are documented here. See [Appendix A: Interface Completeness Audit](#appendix-a-interface-completeness-audit) for the verification method and `scripts/verify-contract-interface.sh` for the automated check.

**Contract**: `DongleContract` (Soroban/Rust)  
**Network**: Stellar  
**Language**: Rust  

### Single-Entity Getter Convention

All single-entity lookup functions (such as `get_project`, `get_collection`, `get_verification`, `get_verification_record`, `get_renewal_request`, `get_assigned_admin`, `get_review`, `get_duplicate_dispute`, `get_proposal`, `get_action`, etc.) return `Option<T>`.
- Returns `Some(entity)` when the record exists.
- Returns `None` when no entity is found for the given ID/key (without raising a contract error).
- Multi-entity/list functions return `Vec<T>` (empty when no entries match).

---

## Table of Contents

1. [Initialization & Admin Management](#initialization--admin-management)
2. [Project Registry](#project-registry)
3. [Project Ownership & Claiming](#project-ownership--claiming)
4. [Project Dependencies](#project-dependencies)
5. [Featured Registry](#featured-registry)
6. [Review Registry](#review-registry)
7. [Verification Registry](#verification-registry)
8. [Verification Renewal](#verification-renewal)
9. [Fee Manager](#fee-manager)
10. [Reporting & Moderation](#reporting--moderation)
11. [Collections](#collections)
12. [Admin Action Log](#admin-action-log)
13. [Dispute Resolution](#dispute-resolution)
14. [TTL Management](#ttl-management)
15. [Contract Configuration](#contract-configuration)
16. [Appendix A: Interface Completeness Audit](#appendix-a-interface-completeness-audit)
17. [Appendix B: Additional Public Functions](#appendix-b-additional-public-functions)

---

## Initialization & Admin Management

### Multi-Sig Governance Workflow

The admin approval threshold determines whether supported administrative actions use a direct call or an on-chain proposal:

| Current threshold | Operational path |
| --- | --- |
| `1` | One authenticated admin may use the direct function. A proposal also works, but is immediately `Approved` because the proposer supplies the first approval. |
| Greater than `1` | Use `create_proposal` -> `approve_proposal` -> `execute_proposal`. The corresponding direct functions return `Unauthorized`. |

This routing applies to the actions represented by `ProposalPayload`: adding or removing an admin, changing the fee configuration and treasury, changing the approval threshold, and approving, rejecting, or revoking a verification. Other admin-only functions that have no `ProposalPayload` variant continue to use their documented direct-call authorization rules.

#### Complete proposal flow

1. Read `get_admin_list` and `get_admin_approval_threshold` so operators know the current eligible signers and quorum.
2. Construct exactly one `ProposalPayload` action and have a current admin call `create_proposal`. The contract authenticates the proposer, assigns the next proposal ID, records the payload and its hash, and automatically adds the proposer as the first approval. The initial status is `Approved` when that one approval meets the current threshold; otherwise it is `Pending`.
3. Distribute the proposal ID and verify the stored payload with `get_proposal` before signing. Each additional current admin calls `approve_proposal` once. Duplicate approvals fail, and approvals can only be added while the proposal is `Pending`. The call that reaches the current threshold changes its status to `Approved`.
4. Re-read the proposal and the current threshold immediately before execution. Any current admin may call `execute_proposal`; the executor does not have to be the proposer or one of the approvers. Execution checks the live threshold again, applies the payload atomically, and changes the status to `Executed`.
5. Confirm the resulting contract state and the proposal's `Executed` status. A proposal cannot be executed twice.

Proposals do not execute automatically when quorum is reached. There is also no proposal expiry or cancellation operation in this interface, so operational tooling should track all non-executed proposals and avoid creating ambiguous duplicates.

#### Threshold changes and existing proposals

The threshold is **not snapshotted** into an `AdminProposal`. Creation, approval, and execution each read the threshold that is current at the time of that call. Consequently:

- Raising the threshold affects every unexecuted proposal. A proposal already marked `Approved` can fail execution when its recorded approval count is below the new threshold. Because `approve_proposal` only accepts `Pending` proposals, no more approvals can be added to that already-`Approved` proposal; it remains blocked until the threshold is lowered sufficiently. Operators should therefore execute ready proposals before raising the threshold, or recreate them after the change.
- Lowering the threshold also affects every unexecuted proposal immediately. A `Pending` proposal whose existing approval count meets the new threshold can be executed even if its stored status has not yet changed to `Approved`, because `execute_proposal` validates the live approval count rather than requiring the `Approved` status. A later valid approval would also refresh a `Pending` proposal to `Approved`.
- Changing the threshold from a value greater than `1` must itself use a `SetThreshold` proposal. The direct `set_admin_approval_threshold` call is available only while the current threshold is `1`.
- A proposed threshold is validated when executed and must be between `1` and the admin count at that moment. Admin-set changes can therefore make a previously valid `SetThreshold` payload fail at execution.

Approval entries are historical addresses stored on the proposal. Execution checks their count, but does not revalidate that every approver is still an admin; only the executor must be a current admin. For predictable governance, complete or replace outstanding proposals as part of any admin rotation.

#### Example: two-of-three administration

```rust
let payload = ProposalPayload::SetThreshold(3);

// admin_1's signature is recorded as approval one.
let proposal_id = create_proposal(env, admin_1, payload)?;

// A distinct current admin supplies approval two, reaching the current 2-of-3 quorum.
approve_proposal(env, admin_2, proposal_id)?;

// Any current admin may execute. This changes the threshold to 3.
execute_proposal(env, admin_3, proposal_id)?;
```

The address arguments shown above must authorize their respective contract invocations.

---

### `initialize`

**Purpose**: Initialize the contract with an initial admin address. This function must be called exactly once before any other operations.

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The initial admin address

**Return Value**: None (void)

**Authorization**: 
- Any address can call this during initialization (typically the contract deployer)
- Only callable once; subsequent calls will fail

**Possible Errors**:
- None (initialization is guarded internally)

**Example**:
```rust
initialize(env, admin_address);
```

---

### `add_admin`

**Purpose**: Add a new admin address to the contract (admin-only operation).

**Parameters**:
- `env` (Env): The contract environment
- `caller` (Address): The admin calling this function (must be an existing admin)
- `new_admin` (Address): The address to promote to admin

**Return Value**: `Result<(), ContractError>`
- Success: `Ok(())`
- Failure: `ContractError`

**Authorization**: 
- Caller must be an existing admin (`is_admin(env, caller)` must return true)

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `AdminNotFound` - Caller address not found in admin list

**Example**:
```rust
add_admin(env, admin_address, new_admin_address)?;
```

---

### `remove_admin`

**Purpose**: Remove an admin address from the contract (admin-only operation).

**Parameters**:
- `env` (Env): The contract environment
- `caller` (Address): The admin calling this function
- `admin_to_remove` (Address): The admin address to remove

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an existing admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `CannotRemoveLastAdmin` - Cannot remove the last admin (contract must maintain at least one admin)
- `AdminNotFound` - Admin to remove not found

**Example**:
```rust
remove_admin(env, caller, admin_to_remove)?;
```

---

### `is_admin`

**Purpose**: Check if an address is an admin.

**Parameters**:
- `env` (Env): The contract environment
- `address` (Address): The address to check

**Return Value**: `bool`
- `true` if the address is an admin
- `false` otherwise

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let is_admin_flag = is_admin(env, some_address);
```

---

### `get_admin_list`

**Purpose**: Retrieve the complete list of all admin addresses.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `Vec<Address>`
- A vector containing all admin addresses

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let admins = get_admin_list(env);
```

---

### `get_admin_count`

**Purpose**: Get the total number of admins in the contract.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `u32`
- The count of admins

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let admin_count = get_admin_count(env);
```

---

### `get_config`

**Purpose**: Return a stable public contract configuration snapshot for frontends and indexers.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `ContractConfig`
- `fee_config`: current fee configuration when set
- `treasury`: current treasury address when set
- `admin_count`: current admin count
- `paused`: current pause state; currently `false` because no pause feature is implemented
- `version`: contract config version string
- public limits for projects, reviews, pagination, tags, social links, verification validity, fee payment expiry, and review update cooldown

**Authorization**:
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let config = get_config(env);
```

---

## Project Registry

### `register_project`

**Purpose**: Register a new project on-chain with metadata.

**Parameters**:
- `env` (Env): The contract environment
- `params` (ProjectRegistrationParams): Registration parameters containing:
  - `owner` (Address): The owner/creator of the project
  - `name` (String): Project name (max length enforced)
  - `slug` (String): URL-friendly project identifier (must be unique)
  - `description` (String): Project description (max length enforced)
  - `category` (String): Project category (max length enforced)
  - `website` (Option<String>): Optional project website URL
  - `logo_cid` (Option<String>): Optional IPFS CID for project logo
  - `metadata_cid` (Option<String>): Optional IPFS CID for extended metadata
  - `tags` (Option<Vec<String>>): Optional tags (max 10 tags, validated)
  - `social_links` (Option<Map<String, String>>): Optional social media links (max 10, validated)
  - `launch_timestamp` (Option<u64>): Optional Unix timestamp of project launch

**Return Value**: `Result<u64, ContractError>`
- Success: `Ok(project_id)` - The unique ID of the registered project
- Failure: `ContractError`

**Authorization**: 
- None (permissionless) - Any address can register a project

**Possible Errors**:
- `ProjectAlreadyExists` - A project with the same slug already exists
- `InvalidProjectName` - Project name format is invalid
- `ProjectNameTooLong` - Project name exceeds maximum length
- `InvalidProjectDesc` - Project description format is invalid
- `ProjectDescTooLong` - Project description exceeds maximum length
- `InvalidCategory` - Category format is invalid
- `CategoryTooLong` - Category exceeds maximum length
- `InvalidWebsite` - Website URL format is invalid
- `WebsiteTooLong` - Website URL exceeds maximum length
- `InvalidLogoCid` - Logo CID format is invalid
- `InvalidMetaCid` - Metadata CID format is invalid
- `InvalidTag` - Tag format is invalid
- `TooManyTags` - More than 10 tags provided
- `InvalidSocialLink` - Social link format is invalid
- `TooManySocialLinks` - More than 10 social links provided
- `MaxProjectsExceeded` - Contract has reached maximum project capacity

**Example**:
```rust
let project_id = register_project(env, ProjectRegistrationParams {
    owner: owner_address,
    name: String::from_slice(&env, "My Project"),
    slug: String::from_slice(&env, "my-project"),
    description: String::from_slice(&env, "A great project"),
    category: String::from_slice(&env, "DeFi"),
    website: Some(String::from_slice(&env, "https://example.com")),
    logo_cid: Some(String::from_slice(&env, "QmXxxx...")),
    metadata_cid: None,
    tags: Some(vec![&env, String::from_slice(&env, "defi")]),
    social_links: None,
    launch_timestamp: None,
})?;
```

---

### `update_project`

**Purpose**: Update project metadata (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `params` (ProjectUpdateParams): Update parameters containing:
  - `project_id` (u64): The ID of the project to update
  - `caller` (Address): The address performing the update (must be project owner)
  - `name` (Option<String>): Optional new project name
  - `slug` (Option<String>): Optional new slug
  - `description` (Option<String>): Optional new description
  - `category` (Option<String>): Optional new category
  - `website` (Option<Option<String>>): Optional new website URL (or None to remove)
  - `logo_cid` (Option<Option<String>>): Optional new logo CID
  - `metadata_cid` (Option<Option<String>>): Optional new metadata CID
  - `tags` (Option<Option<Vec<String>>>): Optional new tags
  - `social_links` (Option<Option<Map<String, String>>>): Optional new social links
  - `launch_timestamp` (Option<Option<u64>>): Optional new launch timestamp

**Return Value**: `Result<Project, ContractError>`
- Success: `Ok(updated_project)` - The updated project data
- Failure: `ContractError`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner
- `ProjectAlreadyExists` - New slug conflicts with existing project
- `InvalidProjectName` - Invalid name format
- `ProjectNameTooLong` - Name exceeds maximum length
- `InvalidProjectDesc` - Invalid description format
- `ProjectDescTooLong` - Description exceeds maximum length
- `InvalidCategory` - Invalid category format
- `CategoryTooLong` - Category exceeds maximum length
- `InvalidWebsite` - Invalid website URL
- `WebsiteTooLong` - Website exceeds maximum length
- `InvalidLogoCid` - Invalid logo CID format
- `InvalidMetaCid` - Invalid metadata CID format
- `InvalidTag` - Invalid tag format
- `TooManyTags` - More than 10 tags
- `InvalidSocialLink` - Invalid social link format
- `TooManySocialLinks` - More than 10 social links

**Example**:
```rust
let updated_project = update_project(env, ProjectUpdateParams {
    project_id: 1,
    caller: owner_address,
    name: Some(String::from_slice(&env, "Updated Project Name")),
    slug: None,
    description: None,
    category: None,
    website: None,
    logo_cid: None,
    metadata_cid: None,
    tags: None,
    social_links: None,
    launch_timestamp: None,
})?;
```

---

### `update_security_contact`

**Purpose**: Update the security contact for a project (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `contact` (Option<String>): Optional security contact email/identifier

**Return Value**: `Result<Project, ContractError>`
- Success: `Ok(updated_project)` - The updated project with security contact
- Failure: `ContractError`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
update_security_contact(env, project_id, owner_address, Some(String::from_slice(&env, "security@example.com")))?;
```

---

### `submit_security_contact_proof`

**Purpose**: Submit proof of security contact ownership via IPFS CID (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `proof_cid` (String): IPFS CID containing proof of security contact

**Return Value**: `Result<Project, ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
submit_security_contact_proof(env, project_id, owner_address, String::from_slice(&env, "QmProof..."))?;
```

---

### `get_security_contact_status`

**Purpose**: Get the security contact verification status for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Result<SecurityContactStatus, ContractError>`
- Contains the current security contact and verification status

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
let status = get_security_contact_status(env, project_id)?;
```

---

### `get_project`

**Purpose**: Retrieve a single project by ID.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The ID of the project to retrieve

**Return Value**: `Option<Project>`
- `Some(project)` if found
- `None` if not found

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(project) = get_project(env, 1) {
    // Use project data
}
```

---

### `get_project_by_slug`

**Purpose**: Retrieve a project by its slug (URL-friendly identifier).

**Parameters**:
- `env` (Env): The contract environment
- `slug` (String): The project slug

**Return Value**: `Option<Project>`
- `Some(project)` if found
- `None` if not found

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(project) = get_project_by_slug(env, String::from_slice(&env, "my-project")) {
    // Use project data
}
```

---

### `list_projects`

**Purpose**: Retrieve projects with pagination, sorted by project ID.

**Parameters**:
- `env` (Env): The contract environment
- `start_id` (u64): The starting project ID for pagination
- `limit` (u32): Maximum number of projects to return

**Return Value**: `Vec<Project>`
- A vector of projects matching the criteria

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let projects = list_projects(env, 0, 10); // Get first 10 projects
```

---

### `get_projects_by_owner`

**Purpose**: Retrieve all projects owned by a specific address.

**Parameters**:
- `env` (Env): The contract environment
- `owner` (Address): The owner address

**Return Value**: `Vec<Project>`
- A vector of all projects owned by the address

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let my_projects = get_projects_by_owner(env, owner_address);
```

---

### `get_owner_project_count`

**Purpose**: Get the count of projects owned by an address.

**Parameters**:
- `env` (Env): The contract environment
- `owner` (Address): The owner address

**Return Value**: `u32`
- The number of projects owned by the address

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let count = get_owner_project_count(env, owner_address);
```

---

### `get_project_count`

**Purpose**: Get the total number of projects in the contract.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `u64`
- The total count of projects

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let total = get_project_count(env);
```

---

### `get_projects_by_ids`

**Purpose**: Retrieve multiple projects by a list of IDs.

**Parameters**:
- `env` (Env): The contract environment
- `ids` (Vec<u64>): A vector of project IDs

**Return Value**: `Vec<Project>`
- A vector of projects found (missing IDs are skipped)

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let projects = get_projects_by_ids(env, vec![&env, 1, 2, 3]);
```

---

### `set_project_region`

**Purpose**: Set or remove an optional region tag for a project (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `region` (Option<String>): Optional region tag (e.g., "US", "EU")

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
set_project_region(env, project_id, owner_address, Some(String::from_slice(&env, "EU")))?;
```

---

### `get_project_region`

**Purpose**: Get the region tag for a project, if set.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Option<String>`
- `Some(region)` if a region tag is set
- `None` if no region tag

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(region) = get_project_region(env, project_id) {
    // Use region data
}
```

---

### `get_project_integrity_hash`

**Purpose**: Get the stored integrity hash for a project, if any.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Option<Bytes>`
- `Some(hash)` if an integrity hash is stored
- `None` if no hash

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(hash) = get_project_integrity_hash(env, project_id) {
    // Verify project integrity
}
```

---

### `list_projects_sorted`

**Purpose**: Retrieve projects sorted by a specified sort mode with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `sort_mode` (ProjectSortMode): The sorting mode (e.g., by rating, by name)
- `start_index` (u64): Zero-based index into the sorted result for pagination
- `limit` (u32): Maximum number of projects to return

**Return Value**: `Vec<Project>`
- A vector of projects sorted by the specified mode

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let sorted_projects = list_projects_sorted(env, ProjectSortMode::Rating, 0, 20);
```

---

### `list_projects_by_status`

**Purpose**: Retrieve projects filtered by verification status with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `status` (VerificationStatus): The verification status to filter by (Unverified, Pending, Verified, Rejected)
- `start_id` (u64): The starting project ID for pagination
- `limit` (u32): Maximum number of projects to return

**Return Value**: `Vec<Project>`
- A vector of projects with the specified status

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let verified_projects = list_projects_by_status(env, VerificationStatus::Verified, 0, 20);
```

---

### `list_projects_by_category`

**Purpose**: Retrieve projects filtered by category with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `category` (String): The category to filter by
- `start_index` (u32): Zero-based index into the category's project ID list for pagination
- `limit` (u32): Maximum number of projects to return

**Return Value**: `Vec<Project>`
- A vector of projects in the specified category

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let defi_projects = list_projects_by_category(env, String::from_slice(&env, "DeFi"), 0, 10);
```

---

### `list_projects_by_tag`

**Purpose**: Retrieve projects filtered by tag with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `tag` (String): The tag to filter by
- `start_index` (u32): Zero-based offset into the project ID scan space for pagination
- `limit` (u32): Maximum number of projects to return

**Return Value**: `Vec<Project>`
- A vector of projects with the specified tag

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let tagged_projects = list_projects_by_tag(env, String::from_slice(&env, "nft"), 0, 10);
```

---

### `archive_project`

**Purpose**: Archive a project (owner or admin can archive, prevents further reviews/verification).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to archive
- `caller` (Address): The address performing the archive

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be project owner or admin

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is neither owner nor admin
- `AlreadyArchived` - Project is already archived

**Example**:
```rust
archive_project(env, project_id, owner_address)?;
```

---

### `reactivate_project`

**Purpose**: Reactivate an archived project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to reactivate
- `caller` (Address): The address performing the reactivation

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be project owner or admin

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is neither owner nor admin
- `ProjectNotArchived` - Project is not archived

**Example**:
```rust
reactivate_project(env, project_id, owner_address)?;
```

---

### `add_maintainer`

**Purpose**: Add a maintainer to a project (owner-only). Maintainers can assist with project management.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `maintainer` (Address): The address to add as maintainer

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
add_maintainer(env, project_id, owner_address, maintainer_address)?;
```

---

### `remove_maintainer`

**Purpose**: Remove a maintainer from a project (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `maintainer` (Address): The maintainer address to remove

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
remove_maintainer(env, project_id, owner_address, maintainer_address)?;
```

---

### `get_maintainers`

**Purpose**: Get the list of maintainers for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<Address>`
- A vector of maintainer addresses

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let maintainers = get_maintainers(env, project_id);
```

---

> **See also:** [`RESERVED_NAMES.md`](./RESERVED_NAMES.md) — feature overview,
> matching semantics, enforcement paths, use cases, and events.

### `add_reserved_name`

**Purpose**: Add a name to the reserved project names list (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin calling
- `name` (String): The name to reserve

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin (via `require_admin_auth`; honours multi-sig quorum)

**Behaviour**:
- Case-insensitive. Idempotent: if an equal name is already reserved the call is a no-op and returns `Ok(())` without emitting an event.
- On success emits `(Symbol("CONFIG"), Symbol("RSVD_ADD"))` and records `ReservedNameAdded` in the admin action log.

**Possible Errors**:
- `Unauthorized` / `AdminOnly` - Caller is not an admin

**Example**:
```rust
add_reserved_name(env, admin_address, String::from_slice(&env, "reserved-name"))?;
```

---

### `remove_reserved_name`

**Purpose**: Remove a name from the reserved list (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin calling
- `name` (String): The name to unreserve

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin (via `require_admin_auth`; honours multi-sig quorum)

**Behaviour**:
- Case-insensitive. Idempotent: if the name is not present the call is a no-op and returns `Ok(())` without emitting an event.
- On success emits `(Symbol("CONFIG"), Symbol("RSVD_REM"))` and records `ReservedNameRemoved` in the admin action log.

**Possible Errors**:
- `Unauthorized` / `AdminOnly` - Caller is not an admin

**Example**:
```rust
remove_reserved_name(env, admin_address, String::from_slice(&env, "reserved-name"))?;
```

---

### `get_reserved_names`

**Purpose**: Get the list of all reserved project names.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `Vec<String>`
- A vector of reserved names

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let reserved = get_reserved_names(env);
```

---

### `is_name_reserved`

**Purpose**: Check if a specific name is reserved.

**Parameters**:
- `env` (Env): The contract environment
- `name` (String): The name to check

**Return Value**: `bool`
- `true` if the name is reserved
- `false` otherwise

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let reserved = is_name_reserved(env, String::from_slice(&env, "some-name"));
```

---

## Project Ownership & Claiming

### `link_project`

**Purpose**: Link two projects together (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The primary project ID
- `caller` (Address): The project owner
- `linked_project_id` (u64): The project ID to link

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the owner of the primary project

**Possible Errors**:
- `ProjectNotFound` - One or both project IDs do not exist
- `Unauthorized` - Caller is not the project owner
- `CannotLinkToSelf` - Cannot link a project to itself
- `AlreadyLinked` - Projects are already linked

**Example**:
```rust
link_project(env, 1, owner_address, 2)?;
```

---

### `unlink_project`

**Purpose**: Unlink two projects (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The primary project ID
- `caller` (Address): The project owner
- `linked_project_id` (u64): The project ID to unlink

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the owner of the primary project

**Possible Errors**:
- `ProjectNotFound` - One or both project IDs do not exist
- `Unauthorized` - Caller is not the project owner
- `CannotLinkToSelf` - Cannot unlink a project from itself

**Example**:
```rust
unlink_project(env, 1, owner_address, 2)?;
```

---

### `get_linked_projects`

**Purpose**: Get all projects linked to a specific project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<u64>`
- A vector of linked project IDs

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let linked_ids = get_linked_projects(env, 1);
```

---

### `initiate_transfer`

**Purpose**: Initiate a project ownership transfer (requires approval from new owner).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to transfer
- `caller` (Address): The current project owner
- `new_owner` (Address): The address of the new owner

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the current project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
initiate_transfer(env, project_id, owner_address, new_owner_address)?;
```

---

### `cancel_transfer`

**Purpose**: Cancel a pending project ownership transfer.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The current project owner

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the current project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner
- `TransferNotFound` - No pending transfer for this project

**Example**:
```rust
cancel_transfer(env, project_id, owner_address)?;
```

---

### `accept_transfer`

**Purpose**: Accept a project ownership transfer (new owner accepts).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The pending new owner

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the pending new owner of the project

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `TransferNotFound` - No pending transfer for this project
- `NotTransferRecip` - Caller is not the pending new owner

**Example**:
```rust
accept_transfer(env, project_id, new_owner_address)?;
```

---

### `claim_contract_address`

**Purpose**: Claim ownership of a contract address associated with a project (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `contract_address` (String): The contract address to claim
- `proof_cid` (String): IPFS CID containing proof of ownership

**Return Value**: `Result<ContractClaimRequest, ContractError>`
- Success: `Ok(claim_request)` - The created contract claim request
- Failure: `ContractError`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
let claim = claim_contract_address(env, project_id, owner_address, String::from_slice(&env, "CC..."), String::from_slice(&env, "QmProof..."))?;
```

---

### `approve_contract_claim`

**Purpose**: Approve a contract address claim (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `contract_address` (String): The contract address being claimed
- `admin` (Address): The admin approving

**Return Value**: `Result<ContractClaimRequest, ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
approve_contract_claim(env, project_id, String::from_slice(&env, "CC..."), admin_address)?;
```

---

### `reject_contract_claim`

**Purpose**: Reject a contract address claim (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `contract_address` (String): The contract address being claimed
- `admin` (Address): The admin rejecting

**Return Value**: `Result<ContractClaimRequest, ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
reject_contract_claim(env, project_id, String::from_slice(&env, "CC..."), admin_address)?;
```

---

### `get_verified_contracts`

**Purpose**: Get all verified contract addresses for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<String>`
- A vector of verified contract addresses

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let contracts = get_verified_contracts(env, project_id);
```

---

### `set_project_claimable`

**Purpose**: Mark a project as claimable by others (owner-only). Used when the original owner no longer maintains it.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `claimable` (bool): True to make claimable, false to revoke

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
set_project_claimable(env, project_id, owner_address, true)?;
```

---

### `submit_claim_request`

**Purpose**: Submit a claim request for a claimable project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to claim
- `claimant` (Address): The address submitting the claim
- `proof_cid` (String): IPFS CID containing proof of stewardship

**Return Value**: `Result<u64, ContractError>`
- Success: `Ok(claim_request_id)` - The ID of the claim request
- Failure: `ContractError`

**Authorization**: 
- Any address can submit a claim for a claimable project

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `InvalidProjectData` - Project is not marked as claimable

**Example**:
```rust
let claim_id = submit_claim_request(env, project_id, claimant_address, String::from_slice(&env, "QmXxxx..."))?;
```

---

### `approve_claim_request`

**Purpose**: Approve a claim request (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `claim_request_id` (u64): The claim request ID to approve
- `admin` (Address): The admin approving the request

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Associated project not found

**Example**:
```rust
approve_claim_request(env, claim_request_id, admin_address)?;
```

---

### `reject_claim_request`

**Purpose**: Reject a claim request (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `claim_request_id` (u64): The claim request ID to reject
- `admin` (Address): The admin rejecting the request

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin

**Example**:
```rust
reject_claim_request(env, claim_request_id, admin_address)?;
```

---

### `get_claim_request`

**Purpose**: Retrieve a single claim request by ID.

**Parameters**:
- `env` (Env): The contract environment
- `claim_request_id` (u64): The claim request ID

**Return Value**: `Option<ClaimRequest>`
- `Some(claim_request)` if found
- `None` if not found

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(claim_req) = get_claim_request(env, claim_id) {
    // Use claim request data
}
```

---

### `get_claim_requests_for_project`

**Purpose**: Get all claim requests for a specific project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<ClaimRequest>`
- A vector of all claim requests for the project

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let claims = get_claim_requests_for_project(env, project_id);
```

---

## Project Dependencies

### `add_project_dependency`

**Purpose**: Add a dependency to a project (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `dependency` (ProjectDependency): The dependency to add containing:
  - `reference` (DependencyRef): Reference to the dependency (project_id, external_cid, or external_url)
  - `label` (Option<String>): Optional label (e.g., "oracle", "token")
  - `metadata_cid` (Option<String>): Optional metadata CID
  - `added_at` (u64): Unix timestamp (usually current time)
  - `updated_at` (u64): Unix timestamp (usually current time)

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist (dependent project, or a `project_id` reference target)
- `Unauthorized` - Caller is not the project owner
- `CannotLinkToSelf` - The `project_id` reference points at the dependent project itself
- `CircularDependency` - The target project already depends (directly or transitively) on this project
- `DependencyDepthExceeded` - The resulting transitive dependency chain would be deeper than `MAX_DEPENDENCY_DEPTH` (5) levels
- `AlreadyLinked` - An identical dependency reference is already registered for this project

**Dependency graph rules**: See [`DEPENDENCY_REGISTRY.md`](./DEPENDENCY_REGISTRY.md) for the circular-reference and depth-limit rules that apply to `project_id` references.

**Example**:
```rust
add_project_dependency(env, project_id, owner_address, ProjectDependency {
    reference: DependencyRef {
        project_id: Some(2),
        external_cid: None,
        external_url: None,
    },
    label: Some(String::from_slice(&env, "oracle")),
    metadata_cid: None,
    added_at: env.ledger().timestamp(),
    updated_at: env.ledger().timestamp(),
})?;
```

---

### `update_project_dependency`

**Purpose**: Update an existing project dependency (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `dependency_key` (DependencyRef): The existing dependency reference to update
- `new_dependency` (ProjectDependency): The updated dependency data

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
update_project_dependency(env, project_id, owner_address, old_ref, new_dependency)?;
```

---

### `remove_project_dependency`

**Purpose**: Remove a dependency from a project (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `dependency_key` (DependencyRef): The dependency reference to remove

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
remove_project_dependency(env, project_id, owner_address, dependency_ref)?;
```

---

### `get_project_dependencies`

**Purpose**: Retrieve all dependencies for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<ProjectDependency>`
- A vector of all project dependencies

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let dependencies = get_project_dependencies(env, project_id);
```

---

## Featured Registry

### `set_featured`

**Purpose**: Set whether a project is featured (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin address
- `project_id` (u64): The project ID to feature/unfeature
- `featured` (bool): True to feature, false to unfeature

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
set_featured(env, admin_address, project_id, true)?;
```

---

### `list_featured_projects`

**Purpose**: Retrieve all featured projects with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `start_index` (u32): The starting index for pagination
- `limit` (u32): Maximum number of projects to return

**Return Value**: `Vec<Project>`
- A vector of featured projects

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let featured = list_featured_projects(env, 0, 20);
```

---

## Review Registry

### `add_review`

**Purpose**: Add or create a review for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID being reviewed
- `reviewer` (Address): The review author
- `rating` (u32): The rating (typically 1-5, validated by contract)
- `comment_cid` (Option<String>): Optional IPFS CID containing the review text

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller (reviewer) can submit review for any project (unless reviews are disabled for that project)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `InvalidRating` - Rating is not in valid range
- `DuplicateReview` - Reviewer has already reviewed this project
- `ReviewsDisabled` - Reviews are disabled for this project
- `ProjectNotArchived` - Cannot review archived projects

**Example**:
```rust
add_review(env, project_id, reviewer_address, 5, Some(String::from_slice(&env, "QmXxxx...")))?;
```

---

### `submit_review`

**Purpose**: Submit a review with content CID (alternative to add_review).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID being reviewed
- `reviewer` (Address): The review author
- `rating` (u32): The rating
- `review_cid` (String): IPFS CID containing the review content

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Reviewer can submit review

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `InvalidRating` - Rating is not valid
- `DuplicateReview` - Reviewer has already reviewed this project
- `ReviewsDisabled` - Reviews disabled for project

**Example**:
```rust
submit_review(env, project_id, reviewer_address, 4, String::from_slice(&env, "QmXxxx..."))?;
```

---

### `update_review`

**Purpose**: Update an existing review (reviewer-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The review author
- `rating` (u32): The new rating
- `comment_cid` (Option<String>): Optional new comment CID

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the reviewer

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `ReviewNotFound` - Review does not exist for this reviewer
- `InvalidRating` - Rating is not valid
- `NotReviewOwner` - Caller is not the reviewer

**Example**:
```rust
update_review(env, project_id, reviewer_address, 3, Some(String::from_slice(&env, "QmYyyy...")))?;
```

---

### `delete_review`

**Purpose**: Delete a review (reviewer-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The review author

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the reviewer

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `ReviewNotFound` - Review does not exist
- `NotReviewOwner` - Caller is not the reviewer

**Example**:
```rust
delete_review(env, project_id, reviewer_address)?;
```

---

### `respond_to_review`

**Purpose**: Project owner responds to a review.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `reviewer` (Address): The reviewer being responded to
- `response` (String): The response text

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `ReviewNotFound` - Review does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
respond_to_review(env, project_id, owner_address, reviewer_address, String::from_slice(&env, "Thank you for the feedback!"))?;
```

---

### `get_review_response`

**Purpose**: Get the project owner's response to a review.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The reviewer

**Return Value**: `Option<String>`
- `Some(response)` if a response exists
- `None` if no response

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(response) = get_review_response(env, project_id, reviewer_address) {
    // Use response text
}
```

---

### `get_review`

**Purpose**: Retrieve a specific review by project and reviewer.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The reviewer address

**Return Value**: `Option<Review>`
- `Some(review)` if found
- `None` if not found

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(review) = get_review(env, project_id, reviewer_address) {
    // Use review data
}
```

---

### `get_review_cid`

**Purpose**: Get the content CID of a review.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The reviewer address

**Return Value**: `Option<String>`
- `Some(cid)` if a review with content CID exists
- `None` otherwise

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(cid) = get_review_cid(env, project_id, reviewer_address) {
    // Fetch full review from IPFS
}
```

---

### `get_project_review_cids`

**Purpose**: Get all review content CIDs for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<(Address, String)>`
- A vector of (reviewer_address, content_cid) pairs

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let review_cids = get_project_review_cids(env, project_id);
// Each entry is (reviewer_address, cid_string)
```

---

### `get_reviews_by_ids`

**Purpose**: Retrieve multiple reviews by a list of (project_id, reviewer) pairs.

**Parameters**:
- `env` (Env): The contract environment
- `ids` (Vec<(u64, Address)>): Vector of (project_id, reviewer_address) tuples

**Return Value**: `Vec<Review>`
- Vector of reviews found (missing combinations are skipped)

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let reviews = get_reviews_by_ids(env, vec![&env, (1, reviewer1), (1, reviewer2)]);
```

---

### `list_reviews`

**Purpose**: List reviews for a project with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `start_index` (u32): Zero-based index into the project's review list for pagination
- `limit` (u32): Maximum number of reviews to return

**Return Value**: `Vec<Review>`
- A vector of reviews for the project

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let reviews = list_reviews(env, project_id, 0, 50);
```

---

### `get_project_stats`

**Purpose**: Get aggregated statistics for a project (review count, average rating).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `ProjectStats`
- Contains:
  - `rating_sum` (u64): Sum of all ratings
  - `review_count` (u32): Number of reviews
  - `average_rating` (u32): Average rating

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let stats = get_project_stats(env, project_id);
let avg = stats.average_rating;
```

---

### `get_stats_batch`

**Purpose**: Get statistics for multiple projects at once.

**Parameters**:
- `env` (Env): The contract environment
- `ids` (Vec<u64>): Vector of project IDs

**Return Value**: `Vec<(u64, ProjectStats)>`
- Vector of (project_id, stats) tuples

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let batch_stats = get_stats_batch(env, vec![&env, 1, 2, 3]);
```

---

### `get_weighted_rating`

**Purpose**: Get the Bayesian weighted rating for a project (scaled by 100).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `u32`
- The weighted rating (e.g., 450 = 4.50)

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let weighted = get_weighted_rating(env, project_id);
```

---

### `get_review_revision_count`

**Purpose**: Get the number of revisions a review has gone through.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The reviewer address

**Return Value**: `u32`
- The number of revisions

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let revisions = get_review_revision_count(env, project_id, reviewer_address);
```

---

### `get_review_history`

**Purpose**: Get revision history for a specific review with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The reviewer address
- `start_index` (u32): Starting index for pagination
- `limit` (u32): Maximum records to return

**Return Value**: `Vec<ReviewRevision>`
- A vector of review revisions

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let history = get_review_history(env, project_id, reviewer_address, 0, 10);
```

---

### `get_review_tombstone`

**Purpose**: Get the deletion tombstone for a review, distinguishing deleted reviews from never-existed.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The reviewer address

**Return Value**: `Option<ReviewTombstone>`
- `Some(tombstone)` if the review was deleted
- `None` if the review never existed or was never deleted

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(tombstone) = get_review_tombstone(env, project_id, reviewer_address) {
    // Review was deleted at tombstone.timestamp
}
```

---

### `list_reviews_sorted`

**Purpose**: List reviews for a project sorted by a specified sort mode with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `start_index` (u32): Zero-based index into the project's review list for pagination
- `limit` (u32): Maximum reviews to return
- `sort_mode` (ReviewSortMode): The sorting mode

**Return Value**: `Vec<Review>`
- A vector of sorted reviews

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let reviews = list_reviews_sorted(env, project_id, 0, 20, ReviewSortMode::Rating);
```

---

### `set_reviews_enabled`

**Purpose**: Enable or disable reviews for a project (owner-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `enabled` (bool): True to enable reviews, false to disable

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

**Example**:
```rust
set_reviews_enabled(env, project_id, owner_address, false)?;
```

---

### `get_reviews_enabled`

**Purpose**: Check if reviews are enabled for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `bool`
- `true` if reviews are enabled
- `false` if disabled

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let enabled = get_reviews_enabled(env, project_id);
```

---

### `report_review`

**Purpose**: Report a review for moderation (spam, abuse, etc.).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The review author
- `reporter` (Address): The address reporting the review

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Any address can report a review

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `ReviewNotFound` - Review does not exist
- `AlreadyReported` - Caller has already reported this review
- `ReviewAlreadyReported` - Review has already been reported

**Example**:
```rust
report_review(env, project_id, reviewer_address, reporter_address)?;
```

---

### `hide_review`

**Purpose**: Hide a review from public view (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The review author
- `admin` (Address): The admin hiding the review

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist
- `ReviewNotFound` - Review does not exist
- `ReviewAlreadyHidden` - Review is already hidden

**Example**:
```rust
hide_review(env, project_id, reviewer_address, admin_address)?;
```

---

### `restore_review`

**Purpose**: Restore a hidden review to public view (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The review author
- `admin` (Address): The admin restoring the review

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist
- `ReviewNotFound` - Review does not exist
- `ReviewNotHidden` - Review is not hidden

**Example**:
```rust
restore_review(env, project_id, reviewer_address, admin_address)?;
```

---

### `admin_delete_review`

**Purpose**: Permanently delete a review (admin-only, irreversible).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The review author
- `admin` (Address): The admin deleting the review

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist
- `ReviewNotFound` - Review does not exist

**Example**:
```rust
admin_delete_review(env, project_id, reviewer_address, admin_address)?;
```

---

## Verification Registry

### `request_verification`

**Purpose**: Request verification of a project (requires fee, if configured).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to verify
- `requester` (Address): The address requesting verification
- `evidence_cid` (String): IPFS CID containing verification evidence

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Any address can request verification for any project
- Project owner typically submits their own projects

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `ProjectTooYoung` - Project age is below minimum required age
- `Unauthorized` - If project is not claimable and caller is not owner
- `InvalidProjectData` - Project data is invalid

**Example**:
```rust
request_verification(env, project_id, requester_address, String::from_slice(&env, "QmXxxx..."))?;
```

---

### `update_verification_evidence`

**Purpose**: Update the verification evidence CID for a pending verification request (project owner only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `caller` (Address): The project owner
- `new_evidence_cid` (String): The new IPFS CID containing updated evidence

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be the project owner
- Updates are allowed only when the request status is `Pending`

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner
- `VerificationNotFound` - No pending verification request

**Example**:
```rust
update_verification_evidence(env, project_id, owner_address, String::from_slice(&env, "QmNewEvidence..."))?;
```

---

### `approve_verification`

**Purpose**: Approve a pending verification request (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin approving

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist
- `VerificationNotFound` - No pending verification request

**Example**:
```rust
approve_verification(env, project_id, admin_address)?;
```

---

### `reject_verification`

**Purpose**: Reject a pending verification request (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin rejecting

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist
- `VerificationNotFound` - No pending verification request

**Example**:
```rust
reject_verification(env, project_id, admin_address)?;
```

---

### `revoke_verification`

**Purpose**: Revoke an active verification (admin-only, typically for compliance).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin revoking
- `reason` (String): Reason for revocation

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist
- `VerificationNotFound` - Project is not verified
- `NotRevocable` - Verification cannot be revoked (already revoked, etc.)

**Example**:
```rust
revoke_verification(env, project_id, admin_address, String::from_slice(&env, "Compliance issue"))?;
```

---

### `get_verification`

**Purpose**: Get the current verification status of a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Option<VerificationRecord>`
- `Some(VerificationRecord)` if found, `None` if not found.
- Contains:
  - `request_id` (u64): ID of the verification request
  - `project_id` (u64): Project ID
  - `requester` (Address): Who requested verification
  - `status` (VerificationStatus): Current status (Unverified, Pending, Verified, Rejected)
  - `evidence_cid` (String): CID of evidence
  - `timestamp` (u64): Request timestamp
  - `fee_amount` (u128): Fee paid
  - `revoke_reason` (Option<String>): Reason if revoked
  - `expires_at` (u64): Expiry timestamp (0 = no expiry)
  - `last_renewed_at` (u64): Last renewal timestamp

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None (returns `None` if verification record does not exist)

**Example**:
```rust
let verification = get_verification(env, project_id);
```

---

### `get_verification_record`

**Purpose**: Get a verification record by its request ID.

**Parameters**:
- `env` (Env): The contract environment
- `request_id` (u64): The verification request ID

**Return Value**: `Result<VerificationRecord, ContractError>`

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- `VerificationNotFound` - No verification record for this request ID

**Example**:
```rust
let record = get_verification_record(env, request_id)?;
```

---

### `get_verifications_batch`

**Purpose**: Get verification records for multiple projects.

**Parameters**:
- `env` (Env): The contract environment
- `ids` (Vec<u64>): Vector of project IDs

**Return Value**: `Vec<(u64, VerificationRecord)>`
- Vector of (project_id, verification_record) tuples

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let verifications = get_verifications_batch(env, vec![&env, 1, 2, 3]);
```

---

### `get_verification_history`

**Purpose**: Get the complete verification history for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<VerificationRecord>`
- A vector of all verification records (past and present)

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let history = get_verification_history(env, project_id);
```

---

### `is_verification_expired`

**Purpose**: Check if a project's verification has expired.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Result<bool, ContractError>`
- `true` if verification has expired
- `false` if not expired or no expiry configured

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `VerificationNotFound` - No verification for project

**Example**:
```rust
let expired = is_verification_expired(env, project_id)?;
```

### `is_verification_expiring_soon`

**Purpose**: Report whether a verification will expire within a caller-supplied renewal-warning threshold.

**Parameters**:
- `project_id` (u64): Project to inspect.
- `threshold_seconds` (u64): Maximum remaining lifetime for the warning.

**Returns**:
- `true` when the verification has a nonzero expiry, is not already expired, and has at most `threshold_seconds` remaining.
- `false` for no-expiry and already-expired records.

```rust
let expiring_soon = is_verification_expiring_soon(env, project_id, 2_592_000)?;
```

---

### `clear_verification_history`

**Purpose**: Admin: prune verification history, keeping the most recent `keep_count` records.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin performing the operation
- `keep_count` (u32): Number of most recent records to keep

**Return Value**: `Result<u32, ContractError>`
- Success: `Ok(removed_count)` - Number of records removed
- Failure: `ContractError`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
let removed = clear_verification_history(env, project_id, admin_address, 5)?;
```

---

### `clear_renewal_history`

**Purpose**: Admin: clear all renewal history records for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin performing the operation

**Return Value**: `Result<u32, ContractError>`
- Success: `Ok(removed_count)` - Number of records removed
- Failure: `ContractError`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
let removed = clear_renewal_history(env, project_id, admin_address)?;
```

---

## Verification Renewal

### `request_renewal`

**Purpose**: Request renewal of an expiring or expired verification.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `requester` (Address): The address requesting renewal
- `evidence_cid` (String): IPFS CID containing updated evidence

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Any address can request (typically project owner)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `VerificationNotFound` - No existing verification to renew

**Example**:
```rust
request_renewal(env, project_id, requester_address, String::from_slice(&env, "QmXxxx..."))?;
```

---

### `approve_renewal`

**Purpose**: Approve a renewal request (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin approving

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
approve_renewal(env, project_id, admin_address)?;
```

---

### `reject_renewal`

**Purpose**: Reject a renewal request (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin rejecting

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
reject_renewal(env, project_id, admin_address)?;
```

---

### `get_renewal_request`

**Purpose**: Get the current renewal request for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Option<VerificationRenewalRecord>`
- `Some(VerificationRenewalRecord)` if found, `None` if not found.
- Contains renewal request details

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None (returns `None` if renewal request does not exist)

**Example**:
```rust
let renewal = get_renewal_request(env, project_id);
```

---

### `get_renewal_history`

**Purpose**: Get renewal history for a project with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `start_index` (u32): Starting index
- `limit` (u32): Maximum records to return

**Return Value**: `Vec<VerificationRenewalRecord>`
- Vector of renewal records

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let renewal_history = get_renewal_history(env, project_id, 0, 10);
```

---

## Verification Assignment

### `assign_verification`

**Purpose**: Admin: assign a pending verification to a specific admin for review.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin performing the assignment
- `assignee` (Address): The admin to assign the verification to

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist
- `VerificationNotFound` - No pending verification for this project

**Example**:
```rust
assign_verification(env, project_id, admin_address, assignee_address)?;
```

---

### `get_assigned_admin`

**Purpose**: Get the admin assigned to review a verification request.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Result<Option<Address>, ContractError>`
- `Ok(Some(admin_address))` if an admin is assigned
- `Ok(None)` if no admin is assigned
- Failure: `ContractError`

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
if let Some(assigned) = get_assigned_admin(env, project_id)? {
    // Assigned admin address
}
```

---

## Fee Manager

### `set_fee`

**Purpose**: Configure fees for contract operations (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin setting fees
- `token` (Option<Address>): Token address (None for Stellar native, Some for specific token)
- `verification_fee` (u128): Fee amount for verification requests
- `registration_fee` (u128): Fee amount for project registration (if enabled)
- `treasury` (Address): Address receiving collected fees

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin

**Example**:
```rust
set_fee(env, admin_address, None, 1000000, 500000, treasury_address)?;
```

---

### `pay_fee`

**Purpose**: Pay required fee for a project operation.

**Parameters**:
- `env` (Env): The contract environment
- `payer` (Address): The address paying the fee
- `project_id` (u64): The project ID the fee is for
- `token` (Option<Address>): Token to pay in (None for native, Some for token contract)

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Payer must authorize the payment

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `FeeConfigNotSet` - Fee configuration not set up
- `TreasuryNotSet` - Treasury address not configured
- `InsufficientFee` - Payment is less than required fee

**Example**:
```rust
pay_fee(env, payer_address, project_id, None)?;
```

---

### `is_fee_paid`

**Purpose**: Check if the fee has been paid for a specific project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `bool`
- `true` if the fee has been paid
- `false` otherwise

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let paid = is_fee_paid(env, project_id);
```

---

### `get_fee_config`

**Purpose**: Get the current fee configuration.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `Result<FeeConfig, ContractError>`
- Contains:
  - `token` (Option<Address>): Token used for fees
  - `verification_fee` (u128): Verification fee amount
  - `registration_fee` (u128): Registration fee amount

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- `FeeConfigNotSet` - No fee configuration has been set

**Example**:
```rust
let fees = get_fee_config(env)?;
```

---

### `pay_registration_fee`

**Purpose**: Pay the required registration fee for a new project.

**Parameters**:
- `env` (Env): The contract environment
- `payer` (Address): The address paying the registration fee
- `token` (Option<Address>): Token to pay in (None for native, Some for token contract)

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Payer must authorize the payment

**Possible Errors**:
- `FeeConfigNotSet` - Fee configuration not set up
- `InsufficientFee` - Payment is less than required fee

**Example**:
```rust
pay_registration_fee(env, payer_address, None)?;
```

---

### `get_fee_payment_details`

**Purpose**: Get fee payment details for a project (payer, amount, token, timestamp).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Option<FeePaymentRecord>`
- `Some(record)` if a payment exists
- `None` if no payment found

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(payment) = get_fee_payment_details(env, project_id) {
    // Use payment details
}
```

---

### `get_reg_fee_payment_details`

**Purpose**: Get registration fee payment details for an address.

**Parameters**:
- `env` (Env): The contract environment
- `address` (Address): The payer address

**Return Value**: `Option<FeePaymentRecord>`
- `Some(record)` if a payment exists
- `None` if no payment found

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(payment) = get_reg_fee_payment_details(env, payer_address) {
    // Use payment details
}
```

---

## Reporting & Moderation

### `report_project`

**Purpose**: Report a project for spam, scams, broken links, or abuse.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to report
- `reporter` (Address): The address reporting
- `reason_cid` (String): IPFS CID containing detailed reason

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Any address can report a project

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `AlreadyReported` - Caller has already reported this project
- `InvalidReportReason` - Reason is invalid

**Example**:
```rust
report_project(env, project_id, reporter_address, String::from_slice(&env, "QmXxxx..."))?;
```

---

### `get_project_reports`

**Purpose**: Get all reports for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<ProjectReport>`
- A vector of all reports, containing:
  - `project_id` (u64): The project
  - `reporter` (Address): Who reported
  - `reason_cid` (String): CID of reason
  - `timestamp` (u64): Report timestamp

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let reports = get_project_reports(env, project_id);
```

---

### `get_project_report_count`

**Purpose**: Get the number of reports for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `u32`
- Count of reports

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let report_count = get_project_report_count(env, project_id);
```

---

### `has_user_reported`

**Purpose**: Check if a user has already reported a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reporter` (Address): The reporter address

**Return Value**: `bool`
- `true` if user has reported, `false` otherwise

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let has_reported = has_user_reported(env, project_id, user_address);
```

---

### `clear_project_reports`

**Purpose**: Clear all reports for a project (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `admin` (Address): The admin clearing reports

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
clear_project_reports(env, project_id, admin_address)?;
```

---

## Collections

### `create_collection`

**Purpose**: Create a new curated collection of projects (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin creating the collection
- `name` (String): Collection name
- `description` (String): Collection description

**Return Value**: `Result<u64, ContractError>`
- Success: `Ok(collection_id)` - The ID of the created collection
- Failure: `ContractError`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `CollectionExists` - Collection with same name already exists

**Example**:
```rust
let collection_id = create_collection(env, admin_address, 
    String::from_slice(&env, "DeFi Projects"),
    String::from_slice(&env, "Top decentralized finance projects"))?;
```

---

### `update_collection`

**Purpose**: Update collection name and description (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin updating
- `collection_id` (u64): The collection ID
- `name` (String): New collection name
- `description` (String): New description

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `CollectionNotFound` - Collection ID does not exist
- `CollectionExists` - New name conflicts with existing collection

**Example**:
```rust
update_collection(env, admin_address, collection_id,
    String::from_slice(&env, "Updated Name"),
    String::from_slice(&env, "Updated description"))?;
```

---

### `delete_collection`

**Purpose**: Delete a collection and its project associations (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin deleting
- `collection_id` (u64): The collection ID

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `CollectionNotFound` - Collection ID does not exist

**Example**:
```rust
delete_collection(env, admin_address, collection_id)?;
```

---

### `add_project_to_collection`

**Purpose**: Add a project to a collection (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin adding
- `collection_id` (u64): The collection ID
- `project_id` (u64): The project ID to add

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `CollectionNotFound` - Collection ID does not exist
- `ProjectNotFound` - Project ID does not exist
- `AlreadyInCollection` - Project already in collection

**Example**:
```rust
add_project_to_collection(env, admin_address, collection_id, project_id)?;
```

---

### `remove_project_from_collection`

**Purpose**: Remove a project from a collection (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin removing
- `collection_id` (u64): The collection ID
- `project_id` (u64): The project ID to remove

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `CollectionNotFound` - Collection ID does not exist
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
remove_project_from_collection(env, admin_address, collection_id, project_id)?;
```

---

### `get_collection`

**Purpose**: Retrieve a collection by ID.

**Parameters**:
- `env` (Env): The contract environment
- `collection_id` (u64): The collection ID

**Return Value**: `Option<Collection>`
- `Some(Collection)` if found, `None` if not found.
- Contains:
  - `id` (u64): Collection ID
  - `name` (String): Collection name
  - `description` (String): Description
  - `created_at` (u64): Creation timestamp
  - `updated_at` (u64): Last update timestamp

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None (returns `None` if collection ID does not exist)

**Example**:
```rust
let collection = get_collection(env, collection_id);
```

---

### `list_collections`

**Purpose**: List all collections with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `start_index` (u32): Starting index
- `limit` (u32): Maximum collections to return

**Return Value**: `Vec<Collection>`
- Vector of collections

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let collections = list_collections(env, 0, 20);
```

---

### `list_collection_projects`

**Purpose**: List project IDs in a collection with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `collection_id` (u64): The collection ID
- `start_index` (u32): Starting index
- `limit` (u32): Maximum project IDs to return

**Return Value**: `Vec<u64>`
- Vector of project IDs

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let project_ids = list_collection_projects(env, collection_id, 0, 50);
```

---

### `get_collection_project_count`

**Purpose**: Get the number of projects in a collection.

**Parameters**:
- `env` (Env): The contract environment
- `collection_id` (u64): The collection ID

**Return Value**: `u32`
- Count of projects in collection

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let count = get_collection_project_count(env, collection_id);
```

---

### `get_collection_count`

**Purpose**: Get the total number of collections.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `u64`
- Total collection count

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let total = get_collection_count(env);
```

---

## Admin Action Log

### `get_admin_action_log_entry`

**Purpose**: Retrieve a single admin action log entry by ID.

**Parameters**:
- `env` (Env): The contract environment
- `log_id` (u64): The log entry ID

**Return Value**: `Option<AdminActionEntry>`
- `Some(entry)` if found, `None` otherwise
- Contains:
  - `id` (u64): Log entry ID
  - `admin` (Address): Admin who performed action
  - `action_type` (AdminActionType): Type of action
  - `target_id` (Option<u64>): Affected project/collection ID
  - `target_address` (Option<Address>): Affected address
  - `timestamp` (u64): Action timestamp
  - `reason_cid` (Option<String>): CID of reason/details

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(entry) = get_admin_action_log_entry(env, log_id) {
    // Use log entry
}
```

---

### `list_admin_actions`

**Purpose**: List admin action log entries with pagination (most recent first).

**Pagination convention**: reverse / most-recent-first offset. `start_index` counts
back from the newest entry, so `start_index = 0` returns the newest `limit` entries
and `start_index = limit` returns the page before that. This differs from the
ID-cursor endpoints (`list_projects`) and the forward index-offset endpoints
(`list_featured_projects`, `list_reviews`); see
[ARCHITECTURE.md §8 Pagination conventions](ARCHITECTURE.md#8-pagination-conventions).

**Parameters**:
- `env` (Env): The contract environment
- `start_index` (u32): Offset back from the newest entry (0 = newest page)
- `limit` (u32): Maximum entries to return

**Return Value**: `Vec<AdminActionEntry>`
- Vector of admin action entries

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let recent_actions = list_admin_actions(env, 0, 100);
```

---

### `get_admin_action_log_count`

**Purpose**: Get the total number of admin action log entries.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `u64`
- Total number of log entries

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let total_actions = get_admin_action_log_count(env);
```

---

## Dispute Resolution

### `open_duplicate_dispute`

**Purpose**: Open a dispute claiming a project is a duplicate of another.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project suspected of being duplicate
- `original_project_id` (u64): The project claimed to be the original
- `creator` (Address): The address opening the dispute
- `evidence_cid` (String): IPFS CID containing evidence of duplication

**Return Value**: `Result<u64, ContractError>`
- Success: `Ok(dispute_id)` - The ID of the created dispute
- Failure: `ContractError`

**Authorization**: 
- Any address can open a dispute

**Possible Errors**:
- `ProjectNotFound` - One or both project IDs do not exist

**Example**:
```rust
let dispute_id = open_duplicate_dispute(env, project_id, original_project_id, creator_address, String::from_slice(&env, "QmXxxx..."))?;
```

---

### `resolve_duplicate_dispute`

**Purpose**: Resolve a duplicate dispute with an action (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `dispute_id` (u64): The dispute ID
- `admin` (Address): The admin resolving
- `action` (DisputeResolutionAction): The resolution action:
  - `Reject` - Reject the dispute claim
  - `ArchiveProject(project_id)` - Archive the suspected duplicate
  - `LinkDuplicates` - Link the two projects as related

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ProjectNotFound` - Associated project not found

**Example**:
```rust
resolve_duplicate_dispute(env, dispute_id, admin_address, DisputeResolutionAction::ArchiveProject(project_id))?;
```

---

### `get_duplicate_dispute`

**Purpose**: Retrieve a duplicate dispute by ID.

**Parameters**:
- `env` (Env): The contract environment
- `dispute_id` (u64): The dispute ID

**Return Value**: `Option<DuplicateDispute>`
- `Some(dispute)` if found, `None` otherwise
- Contains:
  - `id` (u64): Dispute ID
  - `project_id` (u64): Suspected duplicate project
  - `original_project_id` (u64): Claimed original project
  - `creator` (Address): Who opened the dispute
  - `evidence_cid` (String): Evidence CID
  - `status` (DisputeStatus): Pending/Rejected/Resolved
  - `created_at` (u64): Creation timestamp
  - `resolved_at` (u64): Resolution timestamp

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(dispute) = get_duplicate_dispute(env, dispute_id) {
    // Use dispute data
}
```

---

### `get_disputes_for_project`

**Purpose**: Get all duplicate disputes for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `Vec<DuplicateDispute>`
- Vector of all disputes (both as reported project and as original)

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let disputes = get_disputes_for_project(env, project_id);
```

---

## TTL Management

### `extend_project_ttl`

**Purpose**: Extend Time-to-Live for a project and its related data.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: None (void)

**Authorization**: 
- None (permissionless)

**Possible Errors**:
- None

**Example**:
```rust
extend_project_ttl(env, project_id);
```

---

### `extend_review_ttl`

**Purpose**: Extend TTL for a specific review.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `reviewer` (Address): The reviewer address

**Return Value**: None (void)

**Authorization**: 
- None (permissionless)

**Possible Errors**:
- None

**Example**:
```rust
extend_review_ttl(env, project_id, reviewer_address);
```

---

### `extend_admin_ttl`

**Purpose**: Extend TTL for all admin-related data for an admin.

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin address

**Return Value**: None (void)

**Authorization**: 
- None (permissionless)

**Possible Errors**:
- None

**Example**:
```rust
extend_admin_ttl(env, admin_address);
```

---

### `extend_critical_config_ttl`

**Purpose**: Extend TTL for critical contract configuration (admin list, fee config, treasury).

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: None (void)

**Authorization**: 
- None (permissionless)

**Possible Errors**:
- None

**Example**:
```rust
extend_critical_config_ttl(env);
```

---

### `extend_user_ttl`

**Purpose**: Extend TTL for user-related data (owner projects, user reviews).

**Parameters**:
- `env` (Env): The contract environment
- `user` (Address): The user address

**Return Value**: None (void)

**Authorization**: 
- None (permissionless)

**Possible Errors**:
- None

**Example**:
```rust
extend_user_ttl(env, user_address);
```

---

### `extend_verification_ttl`

**Purpose**: Extend TTL for verification data.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: None (void)

**Authorization**: 
- None (permissionless)

**Possible Errors**:
- None

**Example**:
```rust
extend_verification_ttl(env, project_id);
```

---

## Subscription / Follow

### `follow_project`

**Purpose**: Follow (subscribe to) a project for updates.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to follow
- `follower` (Address): The address following the project

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Follower must authorize (self-authenticated)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
follow_project(env, project_id, follower_address)?;
```

---

### `unfollow_project`

**Purpose**: Unfollow (unsubscribe from) a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to unfollow
- `follower` (Address): The address unfollowing

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Follower must authorize (self-authenticated)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
unfollow_project(env, project_id, follower_address)?;
```

---

### `get_follower_count`

**Purpose**: Get the number of followers for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `u32`
- Number of followers

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let count = get_follower_count(env, project_id);
```

---

### `is_following`

**Purpose**: Check if a user is following a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `user` (Address): The user address

**Return Value**: `bool`
- `true` if following, `false` otherwise

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let following = is_following(env, project_id, user_address);
```

---

### `get_project_followers`

**Purpose**: Get the list of followers for a project with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `start_index` (u32): Starting index for pagination
- `limit` (u32): Maximum followers to return

**Return Value**: `Vec<Address>`
- A vector of follower addresses

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let followers = get_project_followers(env, project_id, 0, 20);
```

---

### `get_user_subscriptions`

**Purpose**: Get all projects a user is following with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `user` (Address): The user address
- `start_index` (u32): Starting index for pagination
- `limit` (u32): Maximum subscriptions to return

**Return Value**: `Vec<u64>`
- A vector of project IDs the user follows

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let subscriptions = get_user_subscriptions(env, user_address, 0, 50);
```

---

## Bookmarks

### `bookmark_project`

**Purpose**: Bookmark a project for later reference.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to bookmark
- `user` (Address): The user bookmarking the project

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- User must authorize (self-authenticated)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `AlreadyBookmarked` - Project already bookmarked by this user

**Example**:
```rust
bookmark_project(env, project_id, user_address)?;
```

---

### `unbookmark_project`

**Purpose**: Remove a project bookmark.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to unbookmark
- `user` (Address): The user unbookmarking

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- User must authorize (self-authenticated)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `NotBookmarked` - Project is not bookmarked by this user

**Example**:
```rust
unbookmark_project(env, project_id, user_address)?;
```

---

### `is_bookmarked`

**Purpose**: Check if a user has bookmarked a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `user` (Address): The user address

**Return Value**: `bool`
- `true` if bookmarked, `false` otherwise

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let bookmarked = is_bookmarked(env, project_id, user_address);
```

---

### `get_user_bookmarks`

**Purpose**: Get all bookmarked project IDs for a user with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `user` (Address): The user address
- `start_index` (u32): Starting index for pagination
- `limit` (u32): Maximum bookmarks to return

**Return Value**: `Vec<u64>`
- A vector of bookmarked project IDs

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let bookmarks = get_user_bookmarks(env, user_address, 0, 50);
```

---

## Endorsements

### `endorse_project`

**Purpose**: Endorse a project as a trusted or high-quality project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID to endorse
- `user` (Address): The user endorsing

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- User must authorize (self-authenticated)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `AlreadyEndorsed` - User has already endorsed this project

**Example**:
```rust
endorse_project(env, project_id, user_address)?;
```

---

### `unendorse_project`

**Purpose**: Remove an endorsement from a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `user` (Address): The user unendorsing

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- User must authorize (self-authenticated)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist
- `NotEndorsed` - User has not endorsed this project

**Example**:
```rust
unendorse_project(env, project_id, user_address)?;
```

---

### `get_endorsement_count`

**Purpose**: Get the number of endorsements for a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID

**Return Value**: `u32`
- Number of endorsements

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let count = get_endorsement_count(env, project_id);
```

---

### `has_endorsed`

**Purpose**: Check if a user has endorsed a project.

**Parameters**:
- `env` (Env): The contract environment
- `project_id` (u64): The project ID
- `user` (Address): The user address

**Return Value**: `bool`
- `true` if endorsed, `false` otherwise

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let endorsed = has_endorsed(env, project_id, user_address);
```

---

## Admin Timelock

> **Scheduling delay window.** Every `schedule_*` call validates
> `execution_timestamp` against the current ledger time: it must be at least
> `TIMELOCK_MIN_DELAY` (1 day) and at most `TIMELOCK_MAX_DELAY` (90 days) in
> the future. A past, equal, too-soon, or too-far timestamp returns
> `InvalidInput`. See [`TIMELOCK.md`](./TIMELOCK.md) for the full rules and
> edge cases.

### `schedule_set_fee`

**Purpose**: Schedule a fee configuration change to be executed at a future timestamp (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin scheduling
- `token` (Option<Address>): Token address
- `verification_fee` (u128): New verification fee
- `registration_fee` (u128): New registration fee
- `treasury` (Address): New treasury address
- `execution_timestamp` (u64): Unix timestamp for execution

**Return Value**: `Result<u64, ContractError>`
- Success: `Ok(action_id)` - The scheduled action ID

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `InvalidInput` - `execution_timestamp` is outside the allowed delay window (`< now + TIMELOCK_MIN_DELAY` or `> now + TIMELOCK_MAX_DELAY`)

**Example**:
```rust
let action_id = schedule_set_fee(env, admin_address, None, 1000000, 500000, treasury, future_timestamp)?;
```

---

### `schedule_add_admin`

**Purpose**: Schedule adding a new admin at a future timestamp (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin scheduling
- `new_admin` (Address): The address to promote
- `execution_timestamp` (u64): Unix timestamp for execution

**Return Value**: `Result<u64, ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `InvalidInput` - `execution_timestamp` is outside the allowed delay window (`< now + TIMELOCK_MIN_DELAY` or `> now + TIMELOCK_MAX_DELAY`)

**Example**:
```rust
let action_id = schedule_add_admin(env, admin_address, new_admin_address, future_timestamp)?;
```

---

### `schedule_remove_admin`

**Purpose**: Schedule removing an admin at a future timestamp (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin scheduling
- `admin_to_remove` (Address): The admin to remove
- `execution_timestamp` (u64): Unix timestamp for execution

**Return Value**: `Result<u64, ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `InvalidInput` - `execution_timestamp` is outside the allowed delay window (`< now + TIMELOCK_MIN_DELAY` or `> now + TIMELOCK_MAX_DELAY`)

**Example**:
```rust
let action_id = schedule_remove_admin(env, admin_address, admin_to_remove, future_timestamp)?;
```

---

### `cancel_scheduled_action`

**Purpose**: Cancel a pending scheduled action (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `caller` (Address): The admin cancelling
- `action_id` (u64): The scheduled action ID

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin
- `ActionNotFound` - Action ID does not exist

**Example**:
```rust
cancel_scheduled_action(env, admin_address, action_id)?;
```

---

### `execute_scheduled_set_fee`

**Purpose**: Execute a scheduled fee configuration change after its target timestamp.

**Parameters**:
- `env` (Env): The contract environment
- `caller` (Address): Any address can trigger execution
- `action_id` (u64): The scheduled action ID

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- None (anyone can trigger after the scheduled time)

**Possible Errors**:
- `ActionNotFound` - Action ID does not exist
- `ActionNotReady` - Execution timestamp has not been reached

**Example**:
```rust
execute_scheduled_set_fee(env, caller_address, action_id)?;
```

---

### `execute_scheduled_add_admin`

**Purpose**: Execute a scheduled admin addition after its target timestamp.

**Parameters**:
- `env` (Env): The contract environment
- `caller` (Address): Any address can trigger execution
- `action_id` (u64): The scheduled action ID

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- None (anyone can trigger after the scheduled time)

**Possible Errors**:
- `ActionNotFound` - Action ID does not exist
- `ActionNotReady` - Execution timestamp has not been reached

**Example**:
```rust
execute_scheduled_add_admin(env, caller_address, action_id)?;
```

---

### `execute_scheduled_remove_admin`

**Purpose**: Execute a scheduled admin removal after its target timestamp.

**Parameters**:
- `env` (Env): The contract environment
- `caller` (Address): Any address can trigger execution
- `action_id` (u64): The scheduled action ID

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- None (anyone can trigger after the scheduled time)

**Possible Errors**:
- `ActionNotFound` - Action ID does not exist
- `ActionNotReady` - Execution timestamp has not been reached

**Example**:
```rust
execute_scheduled_remove_admin(env, caller_address, action_id)?;
```

---

### `get_scheduled_action`

**Purpose**: Retrieve a scheduled action by ID.

**Parameters**:
- `env` (Env): The contract environment
- `action_id` (u64): The scheduled action ID

**Return Value**: `Option<TimelockAction>`
- `Some(action)` if found
- `None` if not found

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
if let Some(action) = get_scheduled_action(env, action_id) {
    // Use action data
}
```

---

### `list_scheduled_actions`

**Purpose**: List all scheduled actions with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `start_index` (u32): Starting index for pagination
- `limit` (u32): Maximum actions to return

**Return Value**: `Vec<TimelockAction>`
- A vector of scheduled actions

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let actions = list_scheduled_actions(env, 0, 20);
```

---

### `get_scheduled_action_count`

**Purpose**: Get the total number of scheduled actions.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `u64`
- Total count of scheduled actions

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let count = get_scheduled_action_count(env);
```

---

### Configuration Functions

### `set_min_project_age`

**Purpose**: Set minimum project age before verification is allowed (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin setting the value
- `min_age_seconds` (u64): Minimum age in seconds

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin

**Example**:
```rust
set_min_project_age(env, admin_address, 7 * 24 * 60 * 60)?; // 7 days
```

---

### `get_min_project_age`

**Purpose**: Get the minimum project age setting.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `u64`
- Minimum age in seconds

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let min_age = get_min_project_age(env);
```

---

### `set_verification_duration`

**Purpose**: Set how long a verification is valid (admin-only).

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin setting the value
- `duration_seconds` (u64): Duration in seconds (0 = infinite)

**Return Value**: `Result<(), ContractError>`

**Authorization**: 
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin

**Example**:
```rust
set_verification_duration(env, admin_address, 365 * 24 * 60 * 60)?; // 1 year
```

---

### `get_verification_duration`

**Purpose**: Get the verification validity duration setting.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `u64`
- Duration in seconds

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- None

**Example**:
```rust
let duration = get_verification_duration(env);
```

---

## Common Error Types

The contract uses these error codes consistently (from `ContractError` enum):

| Error | Code | When It Occurs |
|-------|------|----------------|
| `AlreadyInitialized` | 1 | Contract already initialized |
| `NotInitialized` | 2 | Contract not yet initialized |
| `OnlyAdmin` | 3 | Caller is not an admin |
| `ProjectNotFound` | 4 | Project ID doesn't exist |
| `NotProjectOwner` | 5 | Caller is not the project owner |
| `SlugAlreadyExists` | 6 | Project slug already registered |
| `InvalidSlug` | 7 | Invalid project slug format |
| `MaxProjectsExceeded` | 8 | Contract project limit reached |
| `MaxReviewsPerUser` | 9 | User exceeded maximum reviews |
| `MaxReviewsPerProject` | 10 | Project exceeded maximum reviews |
| `ReviewNotFound` | 11 | Review doesn't exist |
| `AlreadyReviewed` | 12 | Reviewer already reviewed project |
| `InvalidCategory` | 13 | Category validation failed |
| `InvalidUrl` | 14 | URL format validation failed |
| `InvalidCid` | 15 | CID format validation failed |
| `InvalidWebsite` | 18 | Website URL validation failed |
| `InvalidLogo` | 19 | Logo data invalid |
| `InvalidMetadata` | 20 | Metadata invalid |
| `InvalidTags` | 21 | Tag format invalid |
| `InvalidSocialLinks` | 22 | Social link format invalid |
| `InvalidLauchTimestamp` | 23 | Launch timestamp invalid |
| `AlreadyMaintainer` | 25 | Address is already a maintainer |
| `NotMaintainer` | 26 | Address is not a maintainer |
| `OnlyMaintainerOrOwner` | 27 | Only maintainer or owner can perform this action |
| `CantRemoveSelf` | 29 | Cannot remove yourself |
| `ProjectAlreadyExists` | 32 | Project slug already registered (alias) |
| `InvalidProjectName` | 33 | Project name validation failed |
| `ProjectNameTooLong` | 34 | Project name exceeds max length |
| `InvalidProjectDesc` | 35 | Project description validation failed |
| `ProjectDescTooLong` | 36 | Description exceeds max length |
| `InvalidProjectData` | 37 | Project data validation failed |
| `InvalidProjectSlug` | 38 | Project slug validation failed |
| `InvalidProjectSlugLen` | 39 | Project slug length invalid |
| `InvalidLogoCid` | 41 | Logo CID validation failed |
| `InvalidMetaCid` | 42 | Metadata CID validation failed |
| `Unauthorized` | 43 | Caller lacks required authorization |
| `AdminOnly` | 44 | Caller is not an admin |
| `AdminNotFound` | 45 | Admin address not found |
| `VerificationNotFound` | 46 | No verification record found |
| `VerificationNotPend` | 47 | Verification is not in pending state |
| `InvalidStatus` | 48 | Invalid verification status value |
| `ProjectTooYoung` | 49 | Project doesn't meet minimum age |
| `VerifiedFieldFrozen` | 50 | Cannot modify a verified project field |
| `AlreadyArchived` | 51 | Project already archived |
| `ProjectNotArchived` | 52 | Project not archived |
| `TransferNotFound` | 53 | No pending transfer found |
| `NotTransferRecip` | 54 | Caller is not transfer recipient |
| `ReservedName` | 43 | Project name is reserved (see [`RESERVED_NAMES.md`](./RESERVED_NAMES.md); canonical codes in [`ERROR_CODES.md`](./ERROR_CODES.md)) |
| `FeeMissing` | 56 | Required fee has not been paid |
| `FeeInvalid` | 57 | Fee configuration is invalid |
| `FeeAlreadyPaid` | 58 | Fee has already been paid |
| `SecurityContactInvalid` | 59 | Security contact validation failed |
| `DuplicateProjectName` | 60 | Normalized project name already exists |

---

## Usage Examples

### Example 1: Project Registration Flow

```rust
// 1. Register a project
let project_id = register_project(env, ProjectRegistrationParams {
    owner: owner_address,
    name: String::from_slice(&env, "MyDeFiToken"),
    slug: String::from_slice(&env, "mydefitoken"),
    description: String::from_slice(&env, "A decentralized finance token"),
    category: String::from_slice(&env, "DeFi"),
    website: Some(String::from_slice(&env, "https://mydefi.com")),
    logo_cid: Some(String::from_slice(&env, "QmXxxx...")),
    metadata_cid: None,
    tags: Some(vec![&env, String::from_slice(&env, "token"), String::from_slice(&env, "defi")]),
    social_links: None,
    launch_timestamp: None,
})?;

// 2. Update project information
update_project(env, ProjectUpdateParams {
    project_id,
    caller: owner_address,
    name: Some(String::from_slice(&env, "MyDeFi Token v2")),
    ..defaults..
})?;

// 3. Add dependencies
add_project_dependency(env, project_id, owner_address, ProjectDependency {
    reference: DependencyRef {
        project_id: Some(other_project_id),
        external_cid: None,
        external_url: None,
    },
    label: Some(String::from_slice(&env, "core-dependency")),
    metadata_cid: None,
    added_at: env.ledger().timestamp(),
    updated_at: env.ledger().timestamp(),
})?;

// 4. Request verification
request_verification(env, project_id, owner_address, String::from_slice(&env, "QmEvidence..."))?;

// 5. Admin approves verification
approve_verification(env, project_id, admin_address)?;

// 6. Retrieve and display project
if let Some(project) = get_project(env, project_id) {
    // Use project data for frontend display
}
```

### Example 2: Review & Rating Flow

```rust
// 1. Add review as a user
add_review(env, project_id, reviewer_address, 4, Some(String::from_slice(&env, "QmReview...")))?;

// 2. Get project statistics
let stats = get_project_stats(env, project_id);
// stats.average_rating, stats.review_count

// 3. Project owner responds to review
respond_to_review(env, project_id, owner_address, reviewer_address, String::from_slice(&env, "Thank you!"))?;

// 4. Get all reviews for a project
let reviews = list_reviews(env, project_id, 0, 50);

// 5. Report an inappropriate review
report_review(env, project_id, reviewer_address, reporter_address)?;

// 6. Admin hides the reported review
hide_review(env, project_id, reviewer_address, admin_address)?;
```

### Example 3: Collection Management (Admin)

```rust
// 1. Create a curated collection
let collection_id = create_collection(env, admin_address, 
    String::from_slice(&env, "Top DeFi Projects"),
    String::from_slice(&env, "Curated list of the best DeFi protocols"))?;

// 2. Add projects to collection
add_project_to_collection(env, admin_address, collection_id, project_id1)?;
add_project_to_collection(env, admin_address, collection_id, project_id2)?;

// 3. Get collection details
let collection = get_collection(env, collection_id)?;

// 4. List projects in collection
let project_ids = list_collection_projects(env, collection_id, 0, 100);
let projects = get_projects_by_ids(env, project_ids);

// 5. Update collection info
update_collection(env, admin_address, collection_id,
    String::from_slice(&env, "Top 10 DeFi Projects"),
    String::from_slice(&env, "Updated curated list"))?;
```

### Example 4: Dispute Resolution

```rust
// 1. User reports duplicate
let dispute_id = open_duplicate_dispute(env, suspect_project_id, original_project_id, reporter_address, String::from_slice(&env, "QmDuplicate..."))?;

// 2. Admin reviews and resolves
if let Some(dispute) = get_duplicate_dispute(env, dispute_id) {
    // Review evidence, then resolve
    resolve_duplicate_dispute(env, dispute_id, admin_address, DisputeResolutionAction::LinkDuplicates)?;
}
```

---

## Security Considerations

1. **Authorization Checks**: All state-modifying operations verify caller authorization
2. **Data Validation**: All inputs are validated for format, length, and content
3. **Unique Constraints**: Project slugs and other identifiers are enforced as unique
4. **Immutable Records**: Verification and review records maintain tamper-proof timestamps
5. **Admin Action Logging**: All admin actions are logged for auditability
6. **Fee Handling**: Fee collection requires proper treasury and token configuration
7. **TTL Management**: Data expiry is managed to prevent bloat on persistent storage

---

## Best Practices

1. **Always check return types**: Functions return `Result` or `Option` - handle both success and failure cases
2. **Validate project ownership**: For owner-only operations, verify ownership before calling
3. **Use pagination**: For list operations, use appropriate `start_id` (project ID cursor) or `start_index` (list offset) with `limit` to avoid timeouts
4. **Cache project data**: Once retrieved, cache project data locally when possible
5. **Monitor admin actions**: Regularly review admin action logs for compliance
6. **Handle duplicates gracefully**: Use dispute resolution for duplicate detection
7. **Extend TTLs proactively**: Call TTL extension functions during maintenance windows
8. **Test with realistic data**: Test with actual project metadata and verification scenarios

---

*This documentation matches the current implementation as of June 2024. For updates, refer to the contract source code in the repository.*

---

## Contract Configuration

Frontends and indexers need a single, stable read of the contract's current configuration. `get_config` returns fees, treasury, admin count, approval threshold, pause state, version, and user-facing limits in one round-trip.

### `get_config`

**Purpose**: Return the aggregated, read-only contract configuration snapshot. Replaces the need to fan out calls to `get_fee_config`, `get_admin_count`, `get_admin_approval_threshold`, etc.

**Parameters**:
- `env` (Env): The contract environment

**Return Value**: `Result<ContractConfigView, ContractError>`
- A fully-populated `ContractConfigView` struct:
  - `version` (`String`): Semantic version of the contract (`CONTRACT_VERSION`).
  - `admin_count` (`u32`): Number of registered admins.
  - `admin_approval_threshold` (`u32`): Approval threshold for multi-admin proposals.
  - `paused` (`bool`): Global pause flag toggled via `set_pause`.
  - `treasury` (`Option<Address>`): Treasury address that receives fees. `None` until `set_fee` is invoked.
  - `fees` (`FeeConfig`): Token + verification + registration fee amounts. Defaults to `None`/`0`/`0` until `set_fee` is invoked.
  - `limits` (`ContractLimits`): User-facing limits surfaced for client validation (max page limit, max projects per user, max reviews per project, max name/description length, verification validity period).

**Authorization**:
- None (read-only, permissionless)

**Possible Errors**: None in normal operation; returns `Ok` even before `set_fee` is called (zero-fee defaults). The "never configured" state is distinguishable from "configured-with-zero-fees" via `treasury: Option<Address>` — only `set_fee` populates it.

**Stability**: The shape of `ContractConfigView` / `ContractLimits` is part of the public contract surface. Only **append** new fields at the end; never reorder, rename, or remove existing fields without bumping `CONTRACT_VERSION`.

**Example**:
```rust
let cfg = get_config(env)?;
println!("contract version {}", cfg.version);
println!("paused = {}", cfg.paused);
```

### `set_pause`

**Purpose**: Admin-only toggle of the global pause flag surfaced by `get_config`. Records an audit-log entry on every transition.

**Parameters**:
- `env` (Env): The contract environment
- `admin` (Address): The admin toggling the flag (must be a current admin)
- `paused` (bool): `true` to pause, `false` to resume

**Return Value**: `Result<bool, ContractError>`
- Returns the pause state **before** the call (so callers can detect transitions without an extra read).
- Other admin entry points in this contract return `()`; the previous-value return is intentional for this method.

**Authorization**:
- Caller must be an admin

**Possible Errors**:
- `AdminOnly` - Caller is not an admin

**Audit logging**:
- Records `AdminActionType::ContractPaused` when toggling `true`.
- Records `AdminActionType::ContractResumed` when toggling `false`.

**Scope**: This method only writes the flag. Enforcement across mutating entry points (`register_project`, `pay_fee`, …) is intentionally out of scope — see the future pause-enforcement ticket. Frontends should treat the flag as advisory for now.

**Example**:
```rust
let _previous = set_pause(env, admin_address, true)?;
```

---

## Appendix A: Interface Completeness Audit

`DongleContract` exposes **198** `pub fn` entry points in
[`lib.rs`](../dongle-smartcontract/src/lib.rs). Prior to this audit, 165 had a
`### \`name\`` section in this file. The remaining 33 are documented in
[Appendix B](#appendix-b-additional-public-functions) below, bringing coverage
to **100%**.

### Verifying coverage

Run the checker (also wired into CI):

```sh
./scripts/verify-contract-interface.sh
```

It fails if any `pub fn` in `lib.rs` lacks a matching `### \`fn\`` heading here,
or if a heading refers to a function that no longer exists. Every documented
function lists its **Parameters**, **Return Value**, **Authorization**,
**Possible Errors**, and (where behaviour is non-obvious) an **Example** and
**Events**.

### Regenerating rustdoc

Structured API docs can also be generated straight from the source doc-comments:

```sh
cargo doc -p dongle-smartcontract --no-deps --document-private-items
# output: target/doc/dongle_smartcontract/struct.DongleContract.html
```

This Markdown file remains the canonical **integrator-facing** reference
(authorization + error semantics + events), which rustdoc does not capture.

---

## Appendix B: Additional Public Functions

These entry points were previously undocumented. Format matches the rest of this
file.

### Multi-Sig Governance (proposals)

See [Multi-Sig Governance Workflow](#multi-sig-governance-workflow) for the
end-to-end flow. The proposal API is an alternative to the direct `add_admin` /
`remove_admin` / `set_fee` calls and is required once the approval threshold is
> 1.

### `create_proposal`

**Purpose**: Open a new admin proposal for a governance action.

**Parameters**:
- `env` (Env)
- `proposer` (Address): must be a current admin; auto-records the proposer's approval
- `payload` (ProposalPayload): one of `AddAdmin(Address)`, `RemoveAdmin(Address)`, `SetFee(Option<Address>, u128, u128, Address)`, `SetThreshold(u32)`, `ApproveVerification(u64)`, `RejectVerification(u64)`, `RevokeVerification(u64, String)`
- `expires_at` (u64): Unix seconds; `0` means no expiry. When non-zero, `execute_proposal` rejects the proposal at/after this time

**Return Value**: `Result<u64, ContractError>` — the new proposal ID

**Authorization**: `proposer` must be an admin (`require_auth`)

**Possible Errors**:
- `AdminOnly` / `Unauthorized` — proposer is not an admin
- `InvalidInput` — malformed payload (e.g. `SetThreshold(0)`)

**Events**: `ProposalCreated { proposal_id, proposer, action_type }`

**Example**:
```rust
let id = create_proposal(env, admin, ProposalPayload::AddAdmin(new_admin), 0)?;
```

### `approve_proposal`

**Purpose**: Record an admin's approval of an open proposal.

**Parameters**: `env`, `admin` (Address), `proposal_id` (u64)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `admin` must be a current admin (`require_auth`)

**Possible Errors**:
- `AdminOnly` / `Unauthorized` — caller is not an admin
- `ProposalExpired` — proposal past `expires_at`
- `InvalidStatus` — proposal is not in the `Pending` state
- `NotFound`-class — no proposal with that ID

**Events**: `ProposalApproved { proposal_id, admin, approvals }`

### `reject_proposal`

**Purpose**: Record an admin's rejection; moves the proposal to `Rejected`.

**Parameters**: `env`, `admin` (Address), `proposal_id` (u64)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `admin` must be a current admin

**Possible Errors**: `AdminOnly` / `Unauthorized`, `InvalidStatus` (already resolved), proposal-not-found

**Events**: `ProposalRejected { proposal_id, admin }`

### `execute_proposal`

**Purpose**: Execute a proposal once it has enough approvals; applies the payload action atomically.

**Parameters**: `env`, `caller` (Address, must be admin), `proposal_id` (u64)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `caller` must be an admin

**Possible Errors**:
- `AdminOnly` / `Unauthorized`
- `InvalidStatus` — not enough approvals, or already executed/rejected
- `ProposalExpired` — past `expires_at`
- `PayloadHashMismatch` — stored payload does not match its recorded hash
- `ThresholdDowngradeRequiresSupermajority` — a `SetThreshold` lowering the threshold needs strictly more approvals than the new threshold
- `CannotRemoveLastAdmin` — `RemoveAdmin` payload would remove the final admin
- any error the underlying action can raise (e.g. `VerificationNotFound` for `ApproveVerification`)

**Events**: `ProposalExecuted { proposal_id, caller }` plus the action's own event

### `get_proposal`

**Purpose**: Fetch a single proposal.

**Parameters**: `env`, `proposal_id` (u64)

**Return Value**: `Option<AdminProposal>` — `None` if the ID is unknown

**Authorization**: None (public read)

**Possible Errors**: None

### `list_proposals`

**Purpose**: Paginated list of proposals.

**Parameters**: `env`, `start_index` (u32, zero-based offset), `limit` (u32, clamped to `MAX_PAGE_LIMIT` = 100)

**Return Value**: `Vec<AdminProposal>` (empty when the offset is past the end)

**Authorization**: None (public read)

**Possible Errors**: None

### `get_admin_approval_threshold`

**Purpose**: Current number of admin approvals required to execute a proposal.

**Parameters**: `env`

**Return Value**: `u32` (defaults to `1`)

**Authorization**: None (public read)

**Possible Errors**: None

### `set_admin_approval_threshold`

**Purpose**: Directly set the multi-sig approval threshold. Only usable while the current threshold is `1`; once > 1 use a `SetThreshold` proposal instead.

**Parameters**: `env`, `caller` (Address, admin), `threshold` (u32, `1..=admin_count`)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `caller` must be an admin

**Possible Errors**:
- `AdminOnly` / `Unauthorized`
- `InvalidInput` — `threshold` is `0` or exceeds the current admin count
- `InvalidStatus` — current threshold is already > 1 (must go through a proposal)

**Events**: `ThresholdChanged { old, new }`

---

### Contract Pause / Emergency Stop

### `pause`

**Purpose**: Halt all non-admin mutating operations.

**Parameters**: `env`, `admin` (Address)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `admin` must be a current admin (`require_auth`)

**Possible Errors**: `AdminOnly` / `Unauthorized`

**Events**: `ContractPaused { admin }`; also records `AdminActionType::ContractPaused` in the admin action log

**Note**: `pause` / `unpause` are the emergency-stop pair. `set_pause` (documented above) is the newer audited toggle that returns the previous state; both write the same flag.

### `unpause`

**Purpose**: Resume normal operation.

**Parameters**: `env`, `admin` (Address)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `admin` must be a current admin

**Possible Errors**: `AdminOnly` / `Unauthorized`

**Events**: `ContractUnpaused { admin }`; records `AdminActionType::ContractResumed`

### `is_paused`

**Purpose**: Whether the contract is currently paused.

**Parameters**: `env`

**Return Value**: `bool`

**Authorization**: None (public read)

**Possible Errors**: None

---

### Project Registry (additional)

### `set_project_lifecycle_status`

**Purpose**: Set a project's lifecycle stage (independent of verification status).

**Parameters**:
- `env`, `project_id` (u64)
- `caller` (Address): must be the project owner or a maintainer
- `status` (ProjectLifecycleStatus): `Active` | `Beta` | `Paused` | `Deprecated` | `Sunset`

**Return Value**: `Result<Project, ContractError>` — the updated project

**Authorization**: `caller` must be the owner or a maintainer (`require_auth`)

**Possible Errors**:
- `ProjectNotFound`
- `Unauthorized` — caller is neither owner nor maintainer
- `ContractPaused`

**Events**: `ProjectLifecycleStatusChanged { project_id, status }`

### `list_projects_by_lifecycle`

**Purpose**: Paginated list of projects filtered by lifecycle status.

**Parameters**: `env`, `status` (ProjectLifecycleStatus), `start_id` (u64), `limit` (u32, clamped to `MAX_PAGE_LIMIT`)

**Return Value**: `Vec<Project>`

**Authorization**: None (public read)

**Possible Errors**: None

### `get_projects_by_tag_batch`

**Purpose**: Fetch projects that carry **any** of the supplied tags, de-duplicated.

**Parameters**: `env`, `tags` (Vec<String>, each validated against `MAX_TAG_LENGTH`), `limit` (u32, clamped to `MAX_PAGE_LIMIT`)

**Return Value**: `Vec<Project>`

**Authorization**: None (public read)

**Possible Errors**: None (invalid tags are ignored rather than erroring)

### `reindex_tags`

**Purpose**: Incrementally (re)build the tag → project index for projects registered before the tag index existed, or after a bulk import.

**Parameters**: `env`, `caller` (Address, admin), `limit` (u32): max projects to process this call

**Return Value**: `Result<u64, ContractError>` — the new watermark (last project ID processed)

**Authorization**: `caller` must be an admin

**Possible Errors**: `AdminOnly` / `Unauthorized`

### `get_tag_index_watermark`

**Purpose**: Highest project ID that `reindex_tags` has processed. When it equals `get_project_count`, the tag index is fully built.

**Parameters**: `env`

**Return Value**: `u64`

**Authorization**: None (public read)

**Possible Errors**: None

---

### Verification Registry (additional)

### `get_pending_verifications`

**Purpose**: Paginated list of verification records still awaiting an admin decision.

**Parameters**: `env`, `start` (u32), `limit` (u32, clamped to `MAX_PAGE_LIMIT`)

**Return Value**: `Vec<VerificationRecord>`

**Authorization**: None (public read)

**Possible Errors**: None

### `get_verification_records_batch`

**Purpose**: Fetch multiple verification records by request ID in one call.

**Parameters**: `env`, `request_ids` (Vec<u64>)

**Return Value**: `Vec<(u64, VerificationRecord)>` — only IDs that exist are returned

**Authorization**: None (public read)

**Possible Errors**: None

### `is_verification_active`

**Purpose**: Whether a project currently holds a non-expired `Verified` status.

**Parameters**: `env`, `project_id` (u64)

**Return Value**: `bool` — `false` for unknown projects, unverified, or expired

**Authorization**: None (public read)

**Possible Errors**: None (infallible variant of `is_verification_expired`, which returns `Result`)

### `renew_verification`

**Purpose**: Admin-driven renewal that extends an existing `Verified` status by the configured verification duration without a fresh request/evidence cycle.

**Parameters**: `env`, `project_id` (u64), `admin` (Address)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `admin` must be a current admin (`require_auth`)

**Possible Errors**:
- `AdminOnly` / `Unauthorized`
- `ProjectNotFound`
- `VerificationNotFound` — no verification record
- `InvalidStatus` — project is not `Verified` (use the renewal-request flow instead)

**Events**: `VerificationRenewed { project_id, new_expiry }`; records `AdminActionType::VerificationRenewalApproved`

---

### Fee Manager (additional)

### `cancel_fee_payment`

**Purpose**: Cancel a fee payment that has been made but not yet consumed by `request_verification`, refunding the payer.

**Parameters**: `env`, `caller` (Address): the original payer or an admin, `project_id` (u64)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `caller` must be the payer or an admin (`require_auth`)

**Possible Errors**:
- `Unauthorized` — caller is neither payer nor admin
- `InvalidStatus` — no outstanding (unconsumed) payment to cancel
- `FeeConfigNotSet`

**Events**: `FeePaymentCancelled { project_id, payer, amount }`

### `claim_fee_refund`

**Purpose**: Withdraw a refund recorded when a verification request was rejected with a fee refund.

**Parameters**: `env`, `caller` (Address): the project owner / original payer, `project_id` (u64)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `caller` must be the recorded refund recipient (`require_auth`)

**Possible Errors**:
- `Unauthorized`
- `NoRefundAvailable` — no refund recorded for this project
- `RefundAlreadyClaimed`
- `ArithmeticOverflow` — checked-math failure computing the payout (defensive)

**Events**: `FeeRefundClaimed { project_id, recipient, amount }`

### `get_fee_refund`

**Purpose**: Read the refund record for a project, if any.

**Parameters**: `env`, `project_id` (u64)

**Return Value**: `Option<FeeRefundRecord>` — `None` when no refund is recorded

**Authorization**: None (public read)

**Possible Errors**: None

### `get_fee_config_history`

**Purpose**: Full ordered history of fee-configuration changes (each `set_fee` appends an entry).

**Parameters**: `env`

**Return Value**: `Vec<FeeConfigHistoryEntry>`

**Authorization**: None (public read)

**Possible Errors**: None

---

### Reporting & Moderation / TTL (additional)

### `extend_projects_ttl`

**Purpose**: Batch-extend the ledger TTL of multiple project entries.

**Parameters**: `env`, `project_ids` (Vec<u64>)

**Return Value**: `Result<u32, ContractError>` — count of entries actually extended

**Authorization**: None (anyone may pay to extend TTL)

**Possible Errors**: `InvalidInput` — batch larger than `MAX_TTL_BATCH_SIZE` (100)

### `extend_reviews_ttl`

**Purpose**: Batch-extend the ledger TTL of multiple review entries.

**Parameters**: `env`, `review_ids` (Vec<(u64, Address)>) — `(project_id, reviewer)` pairs

**Return Value**: `Result<u32, ContractError>` — count extended

**Authorization**: None

**Possible Errors**: `InvalidInput` — batch larger than `MAX_TTL_BATCH_SIZE`

### `get_admin_action_log_by_admin`

**Purpose**: Paginated admin-action-log entries filtered to a single admin.

**Parameters**: `env`, `admin` (Address), `start_index` (u32), `limit` (u32, clamped to `MAX_ADMIN_ACTION_LOG_PAGE` = 100)

**Return Value**: `Vec<AdminActionEntry>`

**Authorization**: None (public read)

**Possible Errors**: None

---

### Project Dependencies (additional)

### `get_project_dependency_count`

**Purpose**: Number of dependencies recorded for a project.

**Parameters**: `env`, `project_id` (u64)

**Return Value**: `u32` (`0` for unknown projects)

**Authorization**: None (public read)

**Possible Errors**: None

---

### Changelog

Project changelog entries are owner-managed pointers to off-chain (IPFS) release
notes, correlated with an optional semantic version string.

### `add_changelog_entry`

**Purpose**: Append a changelog entry for a project.

**Parameters**:
- `env`, `project_id` (u64)
- `owner` (Address): must be the project owner
- `cid` (String): IPFS CID of the changelog content (`MIN_CID_LEN`..=`MAX_CID_LEN`)
- `description` (Option<String>): short title/summary (≤ `MAX_CID_LEN`)
- `version` (Option<String>): semantic version, e.g. `"1.2.3"`
- `changelog_cid` (Option<String>): optional secondary CID (e.g. rendered notes)

**Return Value**: `Result<u64, ContractError>` — the new changelog entry ID

**Authorization**: `owner` must be the project owner (`require_auth`)

**Possible Errors**:
- `ProjectNotFound`
- `Unauthorized` — caller is not the owner
- `InvalidCid` — `cid` / `changelog_cid` fails CID validation
- `InvalidInput` — `description` / `version` over length
- `ContractPaused`

**Events**: `ChangelogEntryAdded { project_id, changelog_id, version }`

### `remove_changelog_entry`

**Purpose**: Delete a changelog entry.

**Parameters**: `env`, `changelog_id` (u64), `owner` (Address)

**Return Value**: `Result<(), ContractError>`

**Authorization**: `owner` must be the owner of the parent project (`require_auth`)

**Possible Errors**:
- `Unauthorized`
- `NotFound`-class — no changelog entry with that ID
- `ContractPaused`

**Events**: `ChangelogEntryRemoved { project_id, changelog_id }`

### `get_changelog_entry`

**Purpose**: Fetch a single changelog entry.

**Parameters**: `env`, `changelog_id` (u64)

**Return Value**: `Option<ChangelogEntry>` — `None` if unknown

**Authorization**: None (public read)

**Possible Errors**: None

### `get_project_changelog`

**Purpose**: Paginated changelog for a project, ordered by `sort_mode`.

**Parameters**:
- `env`, `project_id` (u64)
- `start_index` (u32), `limit` (u32, clamped to `MAX_PAGE_LIMIT`)
- `sort_mode` (ChangelogSortMode): `Newest` (default ordering) or `Oldest`

**Return Value**: `Vec<ChangelogEntry>`

**Authorization**: None (public read)

**Possible Errors**: None

### `get_changelog_count`

**Purpose**: Number of changelog entries for a project.

**Parameters**: `env`, `project_id` (u64)

**Return Value**: `u32`

**Authorization**: None (public read)

**Possible Errors**: None
