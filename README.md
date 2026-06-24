# Dongle-Smartcontract

## Overview

**Dongle** is an open-source smart contract built on the **Stellar network**.
The contract is designed to support a decentralized app discovery and interaction layer, enabling structured registration and management of application metadata on-chain.

Dongle aims to serve as a foundational protocol that frontend applications and indexers can build on to surface, organize, and interact with Stellar-based projects in a transparent and verifiable way.

This repository focuses **only on the smart contract logic**. Frontend interfaces and off-chain indexing are handled separately.

## Quick Start

The smart contract is located in the `dongle-smartcontract/` directory. For comprehensive documentation, usage examples, and API reference, please see the [dongle-smartcontract README](dongle-smartcontract/README.md).

### Prerequisites

- Rust 1.74.0+
- Soroban CLI
- wasm32-unknown-unknown target

### Install Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli --features opt
```

### Build

```bash
cd dongle-smartcontract
make build
# or
cargo build --target wasm32-unknown-unknown --release
```

### Test

```bash
cd dongle-smartcontract
make test
# or
cargo test
```

Run a specific test:

```bash
cargo test test_register_project_success
```

Run tests with output:

```bash
make test-verbose
# or
cargo test -- --nocapture
```

### Deploy to Testnet

```bash
cd dongle-smartcontract

# Configure network (first time only)
soroban network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"

# Create identity (first time only)
soroban keys generate --global alice --network testnet

# Fund account (first time only)
soroban keys fund alice --network testnet

# Deploy
make deploy-testnet
```

## Usage Examples

For detailed usage examples of all contract functions, including:

- **Initialize** - Set up the contract with an admin
- **Register Project** - Register a new project on-chain
- **Update Project** - Update project metadata (owner-only)
- **Add Review** - Submit project reviews with ratings
- **Pay Fee** - Pay verification and registration fees
- **Request Verification** - Request project verification
- **Approve/Reject Verification** - Admin verification actions
- **Project Linking** - Link related projects
- **Featured Projects** - Admin-curated featured lists
- **Project Reporting** - Report projects for moderation
- **Collections** - Admin-curated project collections
- **Project Claiming** - Claim ownership of projects
- **Dependencies** - Track project dependencies
- **Duplicate Disputes** - Report and resolve duplicates
- **And many more...**

See the [comprehensive API documentation](dongle-smartcontract/README.md#usage-examples).

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
* Managing project reviews and ratings
* Handling project verification and renewal
* Supporting project linking and collections
* Providing admin tools for moderation


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

### Extended Functions

* `submit_review` / `add_review` – Submit project reviews
* `request_verification` – Request project verification
* `approve_verification` / `reject_verification` – Admin verification actions
* `link_project` – Link related projects
* `report_project` – Report projects for moderation
* `create_collection` – Create curated project collections
* And many more - see [full API documentation](dongle-smartcontract/README.md)

### Validation

* Prevent duplicate registrations
* Enforce ownership checks
* Validate required fields
* Rating validation (1-5 scale)
* Verification status transitions

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
* Project verification and trust systems
* Review aggregation and rating systems


## Development Status

* Contract structure defined
* Core storage models implemented
* Extended features implemented (reviews, verification, collections, etc.)
* Comprehensive test coverage
* TTL management for persistent storage
* Admin action logging
* Ongoing improvements and testing

This is an **actively evolving open-source project**.

## Features

- **Project Registry**: Register and manage project metadata on-chain
- **Review System**: Submit and manage project reviews with ratings and moderation
- **Verification**: Request, approve, and renew project verification
- **Fee Management**: Configurable fees for operations
- **Access Control**: Owner-based permissions and admin management
- **TTL Management**: Automatic and manual Time-To-Live extension for persistent storage
- **Project Linking**: Link related projects together
- **Featured Projects**: Admin-curated featured project lists
- **Project Reporting**: Report projects for spam, scams, or abuse
- **Collections**: Admin-curated collections of projects
- **Project Claiming**: Claim ownership of unowned projects
- **Dependencies**: Track project dependencies
- **Duplicate Disputes**: Report and resolve duplicate projects

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

## Documentation

- [Smart Contract README](dongle-smartcontract/README.md) - Comprehensive API documentation and usage examples
- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar Developer Portal](https://developers.stellar.org/)
- [Soroban Examples](https://github.com/stellar/soroban-examples)

