# Dongle - Starknet dApp Registry & Ratings

A production-ready Cairo smart contract system for managing a decentralized dApp directory with ratings, reviews, verification, and fee management.

## Overview

Dongle consists of four main contracts:

1. **Registry** - On-chain dApp directory storing minimal fields with rich metadata via IPFS CID
2. **Ratings** - 1-5 star reviews with optional IPFS comment CID, O(1) aggregates, and append-only history
3. **VerificationRegistry** - Project verification system with status management and admin controls
4. **FeeManager** - Verification fee handling with support for STRK and ERC20 tokens

## Features

### Registry Contract
- Add dApps with metadata stored as IPFS CIDs
- Claim/unclaim dApp ownership
- Admin controls for verification and featuring
- Pagination support for listing dApps
- Event emission for all state changes

### Ratings Contract
- 1-5 star rating system
- Optional IPFS CID for review comments
- O(1) aggregate calculations
- Append-only review history
- User can replace their previous review
- Pagination support for listing reviews

### VerificationRegistry Contract
- Project verification status management (None, Pending, Verified, Rejected, Suspended, Revoked)
- Project owners can request verification with evidence
- Admin/verifiers can approve, reject, suspend, or revoke verification
- Comprehensive event emission for all status changes
- Integration with project registry for ownership verification

### FeeManager Contract
- Verification fee payment handling
- Support for STRK native tokens and ERC20 tokens
- Immediate fee forwarding to treasury address
- Admin controls for fee configuration and treasury management
- Fee tracking per project and payer

## Project Structure

```
src/
├── lib.cairo                    # Main module declarations
├── cid.cairo                    # Shared CID utilities
├── access.cairo                 # Shared access control
├── pagination.cairo             # Shared pagination utilities
├── interfaces.cairo             # Registry and Ratings interfaces
├── interfaces/                  # New contract interfaces
│   ├── IVerificationRegistry.cairo
│   └── IFeeManager.cairo
├── registry/                    # Registry contract
│   └── registry.cairo
├── ratings/                     # Ratings contract
│   └── ratings.cairo
├── verification_registry/       # Verification contract
│   └── verification_registry.cairo
└── fee_manager/                 # Fee management contract
    └── fee_manager.cairo

tests/
├── registry_test.cairo          # Registry contract tests
├── ratings_test.cairo           # Ratings contract tests
├── verification_registry_test.cairo  # Verification contract tests
└── fee_manager_test.cairo       # Fee manager contract tests
```

## Building and Testing

### Prerequisites
- Cairo 2.11.4
- Starknet Foundry (snforge)

### Build
```bash
scarb build
```

### Run Tests
```bash
scarb test
```

