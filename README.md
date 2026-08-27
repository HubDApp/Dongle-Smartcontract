# Dongle Smart Contract

**Dongle** is an open-source smart contract built on the **Stellar network** that enables decentralized project discovery and verification on-chain.

## ⚠️ Build Status

**Current Status:** 🔴 **NOT READY FOR DEPLOYMENT** — Build is broken, see [BUILD_STATUS.md](BUILD_STATUS.md) for details.

For deployment readiness assessment, consult [BUILD_STATUS.md](BUILD_STATUS.md) — this is the authoritative source for current project status. Do not rely on other completion/status documents.

## Overview

Dongle serves as a foundational protocol for building transparent, on-chain project registries. It enables:

- **Permissionless project registration** with metadata storage
- **Community reviews** with rating aggregation
- **Admin-managed verification** for trusted projects
- **Access control** based on ownership and admin roles
- **Composable architecture** for indexers and frontend applications

This repository contains the smart contract logic only. Frontend interfaces and off-chain indexing are handled separately.

## Quick Links

For detailed information, refer to:

- **[Architecture Overview](docs/ARCHITECTURE.md)** — Module dependency diagram, data flows, storage layout, and event taxonomy — start here as a new contributor
- **[Smart Contract API & Usage](dongle-smartcontract/README.md)** — Complete API reference, usage examples, and deployment guide
- **[Contract Interface Specification](docs/CONTRACT_INTERFACE.md)** — Detailed function documentation with parameters and error codes
- **[Storage Schema & Keys](docs/STORAGE_SCHEMA.md)** — Storage architecture and persistence management
- **[Admin Rotation & Security](docs/ADMIN_ROTATION_PLAYBOOK.md)** — Operational security guidelines
- **[Admin Timelock](docs/TIMELOCK.md)** — Scheduled admin actions, delay bounds (1–90 days), and edge cases
- **[Event Schema](docs/EVENTS_SCHEMA.md)** — Emitted events for indexing and monitoring
- **[Threat Model](docs/THREAT_MODEL.md)** — Security analysis and risk mitigation
- **[Error Code Reference](docs/ERROR_CODES.md)** — Contract error codes and their meanings
- **[Data Export Guide](docs/DATA_EXPORT_GUIDE.md)** — How indexers reconstruct contract state
- **[Contributing Guidelines](docs/CONTRIBUTING.md)** — How to contribute, test, and submit PRs
- **[Changelog](CHANGELOG.md)** — Release history, breaking changes and feature additions ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/) + SemVer)

## Quick Start

### Prerequisites

- Rust 1.74.0 or later
- Soroban CLI (latest version with `opt` feature)
- wasm32-unknown-unknown target

### Install Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI with optimization support
cargo install --locked soroban-cli --features opt
```

### Build & Test

```bash
cd dongle-smartcontract

# Build the contract
make build
# or: cargo build --target wasm32-unknown-unknown --release

# Run tests
make test
# or: cargo test

# Run tests with output
make test-verbose
# or: cargo test -- --nocapture
```

### Deploy

```bash
# Set your deployer identity
export DEPLOYER_IDENTITY=alice

# Deploy to testnet (automatically saves contract ID to .contract_id)
./scripts/deploy_testnet.sh

# Initialize with an admin
./scripts/initialize.sh

# Invoke a contract method (e.g., register a project)
./scripts/invoke.sh register <owner_address> "My Project" "my-project" "Description" "DeFi"
```

For a comprehensive guide on configuration and scripts, refer to the [dongle-smartcontract README](dongle-smartcontract/README.md).

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

- Providing an on-chain source of truth for project registration
- Enabling transparent project metadata storage
- Allowing permissionless access to registered project data
- Supporting open-source collaboration and extension

## Scope of This Contract

The Dongle smart contract is responsible for:

- Registering projects on-chain
- Storing essential metadata (name, description, links, owner)
- Allowing controlled updates by project owners
- Exposing read methods for frontend and indexers
- Ensuring basic validation and access control
- Managing project reviews and ratings
- Handling project verification and renewal
- Supporting project linking and collections
- Providing admin tools for moderation

## High-Level Architecture

- **Blockchain:** Stellar
- **Smart Contract Framework:** Soroban
- **Language:** Rust
- **Storage:** Soroban persistent storage
- **Access Control:** Address-based ownership

```
Frontend (UI)
   ↓
Dongle Smart Contract (Soroban)
```

## Contract Responsibilities

### Core Functions

- `register_project` – Register a new project on-chain
- `update_project` – Update project metadata (owner-only)
- `get_project` – Fetch a single project’s data
- `list_projects` – Retrieve registered projects (indexer-friendly)

### Extended Functions

- `submit_review` / `add_review` – Submit project reviews
- `request_verification` – Request project verification
- `approve_verification` / `reject_verification` – Admin verification actions
- `link_project` – Link related projects
- `report_project` – Report projects for moderation
- `create_collection` – Create curated project collections
- And many more - see [full API documentation](dongle-smartcontract/README.md)

### Administrator Key Rotation

Operational guidance for secure admin key rotation, incident response, and testnet validation is documented in [docs/ADMIN_ROTATION_PLAYBOOK.md](./docs/ADMIN_ROTATION_PLAYBOOK.md).
### Project Metadata CID Schema

Projects may attach extended off-chain metadata via `metadata_cid` (IPFS). Documents should follow the JSON schema in [`project-metadata.schema.json`](./docs/project-metadata.schema.json).

| | |
|---|---|
| **Schema** | [`project-metadata.schema.json`](./docs/project-metadata.schema.json) |
| **Example** | [`project-metadata.example.json`](./docs/project-metadata.example.json) |
| **Review CID schema** | [`review-cid.schema.json`](./docs/review-cid.schema.json) |
| **Verification evidence schema** | [`verification-evidence.schema.json`](./verification-evidence.schema.json) |
| **Verification evidence example** | [`verification-evidence.example.json`](./verification-evidence.example.json) |

**Required fields:** `version` (semver), `projectName`

**Recommended optional fields:** `description`, `website`, `repository`, `documentation`, `logo`, `banner`, `categories`, `tags`, `socials`, `licenses`, `maintainers`, `createdAt`, `updatedAt`

**Backward compatibility:** Legacy documents that only include `security_contact` (see schema) remain valid. Indexers should treat unknown fields as opaque when validating against older versions.

**Best practices:**

- Pin metadata on IPFS and verify the CID matches on-chain `metadata_cid`
- Bump `version` when making breaking schema changes (use semver)
- Keep on-chain fields (`name`, `description`, `website`) in sync with off-chain metadata
- Legacy documents with only `security_contact` remain valid

## Contract Functions Overview

### Verification Evidence CID Schema

Verification evidence CIDs should point to structured JSON documents with proof
links, screenshots, signatures, attestations, and privacy notes. See
[`docs/VERIFICATION_EVIDENCE.md`](./docs/VERIFICATION_EVIDENCE.md) for the
schema, example document, and safety expectations.

### Validation

- **Admin**: `initialize`, `add_admin`, `remove_admin`, `is_admin`, `get_admin_list`, `get_admin_count`
- **Projects**: `register_project`, `update_project`, `get_project`, `list_projects`, `archive_project`, `reactivate_project`, and more
- **Ownership**: `initiate_transfer`, `accept_transfer`, `set_project_claimable`, `submit_claim_request`, and more
- **Reviews**: `submit_review`, `update_review`, `delete_review`, `report_review`, `hide_review`, and more
- **Verification**: `request_verification`, `approve_verification`, `reject_verification`, `request_renewal`, and more
- **Featured**: `set_featured`, `list_featured_projects`
- **Collections**: `create_collection`, `add_project_to_collection`, `list_collections`, and more
- **Disputes**: `open_duplicate_dispute`, `resolve_duplicate_dispute`, `get_disputes_for_project`
- **Statistics**: `get_project_stats`, `get_project_reports`, `get_project_report_count`

See [CONTRACT_INTERFACE.md](./docs/CONTRACT_INTERFACE.md) for complete documentation, and [dongle-smartcontract/README.md](dongle-smartcontract/README.md) for usage examples.

## Authorization Model

- **Permissionless**: Project registration, reviews, project queries, feature browsing
- **Owner-only**: Project updates, ownership transfers, dependency management, project archiving
- **Admin-only**: Verification approval, collection management, moderation actions, fee configuration
- **None**: All read-only operations are permissionless

## Example Use Cases

- Frontend dApp listing Stellar ecosystem projects
- Indexer tracking newly registered and verified projects
- Open-source project discovery tools
- DAO/community project registries
- Trust and verification systems
- Review aggregation and rating systems

## Development Status

⚠️ **See [BUILD_STATUS.md](BUILD_STATUS.md) for current build and deployment readiness.**

✅ Contract structure defined  
✅ Core storage models implemented  
✅ Extended features (reviews, verification, collections, etc.)  
✅ Comprehensive test coverage  
✅ TTL management for data persistence  
✅ Admin action logging  
✅ Ongoing improvements and testing  

This is an **actively evolving open-source project**.

**Note:** Any "completion" or "ready" documents in the repo history are stale and potentially misleading. Consult [BUILD_STATUS.md](BUILD_STATUS.md) for the actual current state.

## Deployments

Contract deployments are tracked in [deployments.json](./deployments.json). For deployment manifest details and validation procedures, see [DEPLOYMENT.md](./DEPLOYMENT.md).

## Open Source & Contributions

Dongle is open-source and welcomes contributions. You can help by:

- Improving contract logic and security
- Adding tests and coverage
- Enhancing validation and error handling
- Reviewing security assumptions
- Improving documentation

Please open an issue or pull request for proposed changes.

## Why This Project Matters

Dongle promotes:

- Transparency in project discovery
- Decentralized ownership of ecosystem data
- Composable infrastructure for Stellar builders
- Open collaboration through smart contracts

## Documentation

- [Architecture Overview](docs/ARCHITECTURE.md) - Module structure, dependency diagram, data flows, storage layout, event taxonomy
- [Smart Contract README](dongle-smartcontract/README.md) - Comprehensive API documentation and usage examples
- [EVENTS_SCHEMA.md](docs/EVENTS_SCHEMA.md) - Event topic and data schema reference for indexers
- [THREAT_MODEL.md](docs/THREAT_MODEL.md) - Security threat model and mitigation reference
- [ERROR_CODES.md](docs/ERROR_CODES.md) - Contract error codes reference
- [DATA_EXPORT_GUIDE.md](docs/DATA_EXPORT_GUIDE.md) - Data export guide for indexers
- [STORAGE_INDEXES.md](docs/STORAGE_INDEXES.md) - Storage index size strategy and read pagination
- [CONTRACT_INTERFACE.md](docs/CONTRACT_INTERFACE.md) - Full contract function documentation
- [CONTRIBUTING.md](docs/CONTRIBUTING.md) - Contribution guidelines and PR process
- [CHANGELOG.md](CHANGELOG.md) - Release history, breaking changes and feature additions (Keep a Changelog 1.1.0 format; validated in CI by `scripts/validate_changelog.py`)
- [review-cid.schema.json](docs/review-cid.schema.json) - Off-chain JSON schema for review content CIDs
- [review-cid.example.json](docs/review-cid.example.json) - Valid off-chain JSON review example
- [verification-evidence.schema.json](verification-evidence.schema.json) - Off-chain JSON schema for verification evidence CIDs
- [verification-evidence.example.json](verification-evidence.example.json) - Valid verification evidence example
- [Verification Evidence Guide](docs/VERIFICATION_EVIDENCE.md) - Privacy, safety, proof-link, attestation, and signature guidance
- [Storage Schema Reference](docs/STORAGE_SCHEMA.md) — canonical storage keys, read/write mapping, index consistency rules, and migration guidance.
- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar Developer Portal](https://developers.stellar.org/)
- [Soroban Examples](https://github.com/stellar/soroban-examples)
