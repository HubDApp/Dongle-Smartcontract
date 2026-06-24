# Dongle-Smartcontract

## Overview

**Dongle** is an open-source smart contract built on the **Stellar network**.
The contract is designed to support a decentralized app discovery and interaction layer, enabling structured registration and management of application metadata on-chain.

Dongle aims to serve as a foundational protocol that frontend applications and indexers can build on to surface, organize, and interact with Stellar-based projects in a transparent and verifiable way.

This repository focuses **only on the smart contract logic**. Frontend interfaces and off-chain indexing are handled separately.


## Problem Statement

Discoverability and trust remain challenges in decentralized ecosystems. Many projects rely on off-chain listings, centralized platforms, or unverifiable data sources.

Dongle addresses this by:

* Providing an on-chain source of truth for project registration
* Enabling transparent project metadata storage
* Allowing permissionless access to registered project data
* Supporting open-source collaboration and extension


## Scope of This Contract

The Dongle smart contract is responsible for:

* Registering projects on-chain
* Storing essential metadata (name, description, links, owner)
* Allowing controlled updates by project owners
* Exposing read methods for frontend and indexers
* Ensuring basic validation and access control


## High-Level Architecture

* **Blockchain:** Stellar
* **Smart Contract Framework:** Soroban
* **Language:** Rust
* **Storage:** Soroban persistent storage
* **Access Control:** Address-based ownership

```
Frontend (UI)
   ↓
Dongle Smart Contract (Soroban)
```


## Contract Responsibilities

### Core Functions

* `register_project` – Register a new project on-chain
* `update_project` – Update project metadata (owner-only)
* `get_project` – Fetch a single project’s data
* `list_projects` – Retrieve registered projects (indexer-friendly)

### Validation

* Prevent duplicate registrations
* Enforce ownership checks
* Validate required fields
---

## 📚 Comprehensive Contract Interface Documentation

For complete documentation of all contract functions, including parameters, return values, authorization requirements, and possible errors, please see **[CONTRACT_INTERFACE.md](./CONTRACT_INTERFACE.md)**.

### Quick Navigation

The contract is organized into logical sections:

#### Core Functions
- **[Initialization & Admin Management](./CONTRACT_INTERFACE.md#initialization--admin-management)** – Contract setup, admin management, and access control
- **[Project Registry](./CONTRACT_INTERFACE.md#project-registry)** – Project registration, updates, and retrieval
- **[Project Ownership & Claiming](./CONTRACT_INTERFACE.md#project-ownership--claiming)** – Project transfers, claiming, and ownership
- **[Project Dependencies](./CONTRACT_INTERFACE.md#project-dependencies)** – Managing project dependencies and relationships

#### Features
- **[Featured Registry](./CONTRACT_INTERFACE.md#featured-registry)** – Curated project features
- **[Review Registry](./CONTRACT_INTERFACE.md#review-registry)** – Reviews, ratings, and owner responses
- **[Verification Registry](./CONTRACT_INTERFACE.md#verification-registry)** – Project verification and validation
- **[Verification Renewal](./CONTRACT_INTERFACE.md#verification-renewal)** – Verification renewal processes
- **[Collections](./CONTRACT_INTERFACE.md#collections)** – Curated collections of projects (admin-only)

#### Operations
- **[Fee Manager](./CONTRACT_INTERFACE.md#fee-manager)** – Fee configuration and collection
- **[Reporting & Moderation](./CONTRACT_INTERFACE.md#reporting--moderation)** – Project and review reporting
- **[Dispute Resolution](./CONTRACT_INTERFACE.md#dispute-resolution)** – Duplicate project dispute handling
- **[Admin Action Log](./CONTRACT_INTERFACE.md#admin-action-log)** – Audit trail of admin actions
- **[TTL Management](./CONTRACT_INTERFACE.md#ttl-management)** – Data lifetime management

### All Public Functions

The contract exposes the following public function categories:

| Category | Functions |
|----------|-----------|
| Admin | `initialize`, `add_admin`, `remove_admin`, `is_admin`, `get_admin_list`, `get_admin_count` |
| Projects | `register_project`, `update_project`, `get_project`, `get_project_by_slug`, `list_projects`, `get_projects_by_owner`, `archive_project`, `reactivate_project`, `get_linked_projects`, `link_project`, `unlink_project` |
| Ownership | `initiate_transfer`, `cancel_transfer`, `accept_transfer`, `set_project_claimable`, `submit_claim_request`, `approve_claim_request`, `reject_claim_request` |
| Reviews | `add_review`, `update_review`, `delete_review`, `submit_review`, `respond_to_review`, `get_review`, `list_reviews`, `report_review`, `hide_review`, `restore_review`, `admin_delete_review` |
| Verification | `request_verification`, `approve_verification`, `reject_verification`, `revoke_verification`, `request_renewal`, `approve_renewal`, `reject_renewal` |
| Featured | `set_featured`, `list_featured_projects` |
| Collections | `create_collection`, `update_collection`, `delete_collection`, `add_project_to_collection`, `remove_project_from_collection`, `list_collections` |
| Disputes | `open_duplicate_dispute`, `resolve_duplicate_dispute`, `get_duplicate_dispute`, `get_disputes_for_project` |
| Statistics | `get_project_stats`, `get_stats_batch`, `get_project_reports`, `get_project_report_count` |

### Key Error Types

The contract uses error codes for different failure scenarios. See [Common Error Types](./CONTRACT_INTERFACE.md#common-error-types) for complete error documentation.

### Authorization Model

- **Permissionless**: Project registration, reviews, project queries
- **Owner-only**: Project updates, ownership transfers, dependency management
- **Admin-only**: Verification approval, collection management, moderation actions, configuration
- **None**: All read-only operations are permissionless

### Best Practices for Integration

1. **Always handle Result types** – Functions may fail; check error codes
2. **Verify ownership** – For sensitive operations, confirm caller authorization
3. **Use pagination** – Large queries should use start/limit parameters
4. **Check project status** – Archived projects behave differently
5. **Monitor verification status** – Use verification checks before trust decisions
6. **Manage TTLs** – Keep important data alive with TTL extension calls

---
## Example Use Cases

* A frontend dApp listing Stellar ecosystem projects
* An indexer tracking newly registered applications
* Open-source contributors building discovery tools
* DAO or community-driven project registries


## Development Status

* Contract structure defined
* Core storage models implemented
* Ongoing improvements and testing
* Extended metadata & governance features

This is an **actively evolving open-source project**.

## Open Source & Contributions

Dongle is open-source and welcomes contributions.

You can contribute by:

* Improving contract structure
* Adding tests
* Enhancing validation logic
* Reviewing security assumptions
* Improving documentation

Please open an issue or pull request for any proposed changes.

## Why This Project Matters

Dongle promotes:

* Transparency in project discovery
* Decentralized ownership of ecosystem data
* Composable infrastructure for Stellar builders
* Open collaboration through smart contracts



