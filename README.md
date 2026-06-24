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

