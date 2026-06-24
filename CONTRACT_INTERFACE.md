# Dongle Smart Contract Interface Documentation

## Overview

This document provides comprehensive documentation of all public contract functions in the Dongle smart contract. Each function includes its purpose, parameters, return values, authorization requirements, and possible errors.

**Contract**: `DongleContract` (Soroban/Rust)  
**Network**: Stellar  
**Language**: Rust  

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

---

## Initialization & Admin Management

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
- `start_id` (u32): The starting index for pagination
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
- `start_id` (u32): The starting index for pagination
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
- `ProjectNotFound` - Project ID does not exist
- `Unauthorized` - Caller is not the project owner

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
- `start` (u32): The starting index for pagination
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
- `start_id` (u32): The starting index for pagination
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

**Return Value**: `Result<VerificationRecord, ContractError>`
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
- `ProjectNotFound` - Project ID does not exist
- `VerificationNotFound` - No verification record for this project

**Example**:
```rust
let verification = get_verification(env, project_id)?;
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

**Return Value**: `Result<VerificationRenewalRecord, ContractError>`
- Contains renewal request details

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- `ProjectNotFound` - Project ID does not exist

**Example**:
```rust
let renewal = get_renewal_request(env, project_id)?;
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

**Return Value**: `Result<Collection, ContractError>`
- Contains:
  - `id` (u64): Collection ID
  - `name` (String): Collection name
  - `description` (String): Description
  - `created_at` (u64): Creation timestamp
  - `updated_at` (u64): Last update timestamp

**Authorization**: 
- None (read-only, permissionless)

**Possible Errors**:
- `CollectionNotFound` - Collection ID does not exist

**Example**:
```rust
let collection = get_collection(env, collection_id)?;
```

---

### `list_collections`

**Purpose**: List all collections with pagination.

**Parameters**:
- `env` (Env): The contract environment
- `start` (u32): Starting index
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
- `start` (u32): Starting index
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

**Parameters**:
- `env` (Env): The contract environment
- `start` (u32): Starting index
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

The contract uses these error codes consistently:

| Error | Code | When It Occurs |
|-------|------|----------------|
| `ProjectNotFound` | 1 | Project ID doesn't exist |
| `Unauthorized` | 2 | Caller lacks required authorization |
| `ProjectAlreadyExists` | 3 | Project slug already registered |
| `InvalidRating` | 4 | Rating outside valid range |
| `ReviewNotFound` | 5 | Review doesn't exist |
| `DuplicateReview` | 6 | Reviewer already reviewed project |
| `NotReviewOwner` | 7 | Caller is not the review author |
| `VerificationNotFound` | 8 | No verification record found |
| `InvalidStatus` | 9 | Invalid status value |
| `AdminOnly` | 10 | Caller is not an admin |
| `FeeConfigNotSet` | 11 | Fee configuration not initialized |
| `TreasuryNotSet` | 12 | Treasury address not set |
| `InsufficientFee` | 13 | Payment below required fee |
| `InvalidProjectData` | 14 | Project data validation failed |
| `ProjectNameTooLong` | 15 | Project name exceeds max length |
| `InvalidNameFormat` | 16 | Project name format invalid |
| `CannotRemoveLastAdmin` | 17 | Cannot remove only remaining admin |
| `ProjectTooYoung` | 18 | Project doesn't meet minimum age |
| `InvalidTag` | 19 | Tag format invalid |
| `TooManyTags` | 20 | More than 10 tags provided |
| `InvalidSocialLink` | 21 | Social link format invalid |
| `TooManySocialLinks` | 22 | More than 10 social links provided |
| `AlreadyReported` | 23 | Address already reported this |
| `InvalidReportReason` | 24 | Report reason format invalid |
| `AdminNotFound` | 25 | Admin address not found |
| `InvalidProjectName` | 26 | Project name validation failed |
| `InvalidProjectDesc` | 27 | Project description validation failed |
| `InvalidCategory` | 28 | Category validation failed |
| `ProjectDescTooLong` | 29 | Description exceeds max length |
| `MaxProjectsExceeded` | 30 | Contract project limit reached |
| `InvalidWebsite` | 31 | Website URL validation failed |
| `InvalidLogoCid` | 32 | Logo CID validation failed |
| `InvalidMetaCid` | 33 | Metadata CID validation failed |
| `CategoryTooLong` | 34 | Category exceeds max length |
| `WebsiteTooLong` | 35 | Website URL exceeds max length |
| `NotRevocable` | 36 | Verification cannot be revoked |
| `TransferNotFound` | 37 | No pending transfer found |
| `NotTransferRecip` | 38 | Caller is not transfer recipient |
| `ReviewsDisabled` | 39 | Reviews disabled for project |
| `ReviewAlreadyReported` | 40 | Review already reported |
| `ReviewAlreadyHidden` | 41 | Review already hidden |
| `ReviewNotHidden` | 42 | Review is not hidden |
| `AlreadyArchived` | 43 | Project already archived |
| `ProjectNotArchived` | 44 | Project not archived |
| `ReportsCleared` | 45 | Reports have been cleared |
| `CollectionNotFound` | 46 | Collection ID doesn't exist |
| `CollectionExists` | 47 | Collection already exists |
| `AlreadyInCollection` | 48 | Project already in collection |
| `AlreadyLinked` | 49 | Projects already linked |
| `CannotLinkToSelf` | 50 | Cannot link project to itself |

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
3. **Use pagination**: For list operations, use appropriate start_id/limit to avoid timeouts
4. **Cache project data**: Once retrieved, cache project data locally when possible
5. **Monitor admin actions**: Regularly review admin action logs for compliance
6. **Handle duplicates gracefully**: Use dispute resolution for duplicate detection
7. **Extend TTLs proactively**: Call TTL extension functions during maintenance windows
8. **Test with realistic data**: Test with actual project metadata and verification scenarios

---

*This documentation matches the current implementation as of June 2024. For updates, refer to the contract source code in the repository.*
