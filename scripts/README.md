# Dongle Smart Contract

**Dongle** is an open-source smart contract built on the **Stellar network** that enables decentralized project discovery and verification on-chain.

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

- **[Smart Contract API & Usage](dongle-smartcontract/README.md)** — Complete API reference, usage examples, and deployment guide
- **[Contract Interface Specification](CONTRACT_INTERFACE.md)** — Detailed function documentation with parameters and error codes
- **[Storage Schema & Keys](docs/STORAGE_SCHEMA.md)** — Storage architecture and persistence management
- **[Admin Rotation & Security](docs/ADMIN_ROTATION_PLAYBOOK.md)** — Operational security guidelines
- **[Event Schema](EVENTS_SCHEMA.md)** — Emitted events for indexing and monitoring
- **[Threat Model](THREAT_MODEL.md)** — Security analysis and risk mitigation

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

For detailed deployment instructions with environment variables and script reference, see the [Smart Contract API Guide](dongle-smartcontract/README.md#deploy-and-invoke-via-scripts).

## Key Features

| Feature | Description |
|---------|-------------|
| **Project Registry** | Register and manage project metadata on-chain |
| **Reviews & Ratings** | Submit community reviews with rating aggregation |
| **Verification** | Admin-managed project verification with renewal support |
| **Fee Management** | Configurable fees for operations (token or native XLM) |
| **Access Control** | Owner-based permissions and admin role management |
| **TTL Management** | Automatic and manual time-to-live extension for data persistence |
| **Project Linking** | Link related projects together (e.g., mainnet/testnet pairs) |
| **Moderation** | Report projects and reviews; admin hide/delete capabilities |
| **Collections** | Admin-curated project collections |
| **Project Claiming** | Claim ownership of unclaimed projects |
| **Dependencies** | Track project dependencies and relationships |
| **Duplicate Resolution** | Report and resolve duplicate project disputes |

## Project Metadata (Off-Chain)

Projects can attach extended metadata via IPFS CIDs. Follow the JSON schema:

| Schema | Purpose |
|--------|---------|
| [`project-metadata.schema.json`](./project-metadata.schema.json) | Project metadata structure |
| [`project-metadata.example.json`](./project-metadata.example.json) | Example valid document |
| [`review-cid.schema.json`](./review-cid.schema.json) | Review content structure |
| [`review-cid.example.json`](./review-cid.example.json) | Example review document |

**Key guidelines:**
- Pin metadata on IPFS and verify the CID matches on-chain `metadata_cid`
- Bump `version` when making breaking schema changes (use semver)
- Keep on-chain fields (`name`, `description`, `website`) in sync with off-chain metadata
- Legacy documents with only `security_contact` remain valid

## Contract Functions Overview

The contract exposes 100+ functions organized by domain:

- **Admin**: `initialize`, `add_admin`, `remove_admin`, `is_admin`, `get_admin_list`, `get_admin_count`
- **Projects**: `register_project`, `update_project`, `get_project`, `list_projects`, `archive_project`, `reactivate_project`, and more
- **Ownership**: `initiate_transfer`, `accept_transfer`, `set_project_claimable`, `submit_claim_request`, and more
- **Reviews**: `submit_review`, `update_review`, `delete_review`, `report_review`, `hide_review`, and more
- **Verification**: `request_verification`, `approve_verification`, `reject_verification`, `request_renewal`, and more
- **Featured**: `set_featured`, `list_featured_projects`
- **Collections**: `create_collection`, `add_project_to_collection`, `list_collections`, and more
- **Disputes**: `open_duplicate_dispute`, `resolve_duplicate_dispute`, `get_disputes_for_project`
- **Statistics**: `get_project_stats`, `get_project_reports`, `get_project_report_count`

See [CONTRACT_INTERFACE.md](./CONTRACT_INTERFACE.md) for complete documentation, and [dongle-smartcontract/README.md](dongle-smartcontract/README.md) for usage examples.

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

✅ Contract structure defined  
✅ Core storage models implemented  
✅ Extended features (reviews, verification, collections, etc.)  
✅ Comprehensive test coverage  
✅ TTL management for data persistence  
✅ Admin action logging  
✅ Ongoing improvements and testing  

This is an **actively evolving open-source project**.

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

- **Transparency**: On-chain, verifiable source of truth for projects
- **Decentralization**: Community-owned ecosystem data
- **Composability**: Reusable infrastructure for Stellar builders
- **Open Collaboration**: Standards-based smart contract protocol

## Documentation Index

| Document | Purpose |
|----------|---------|
| [dongle-smartcontract/README.md](dongle-smartcontract/README.md) | API reference, usage examples, deployment guide |
| [CONTRACT_INTERFACE.md](CONTRACT_INTERFACE.md) | Complete function documentation with error codes |
| [EVENTS_SCHEMA.md](EVENTS_SCHEMA.md) | Event topics and payloads for indexers |
| [THREAT_MODEL.md](THREAT_MODEL.md) | Security analysis and mitigation strategies |
| [docs/STORAGE_SCHEMA.md](docs/STORAGE_SCHEMA.md) | Storage keys and persistence architecture |
| [docs/ADMIN_ROTATION_PLAYBOOK.md](docs/ADMIN_ROTATION_PLAYBOOK.md) | Admin key rotation and incident response |
| [docs/LOGO_ASSET_GUIDELINES.md](docs/LOGO_ASSET_GUIDELINES.md) | Logo CID best practices |
| [Soroban Documentation](https://soroban.stellar.org/docs) | Soroban contract development guide |
| [Stellar Developer Portal](https://developers.stellar.org/) | Stellar network documentation |
| [Soroban Examples](https://github.com/stellar/soroban-examples) | Community contract examples |
