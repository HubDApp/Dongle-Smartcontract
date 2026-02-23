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



