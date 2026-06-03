# Dongle Smart Contract

A Soroban smart contract for decentralized project registry, reviews, and verification on the Stellar network.

## Quick Start

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
make build
# or
cargo build --target wasm32-unknown-unknown --release
```

### Test

```bash
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

---

## Usage Examples

All examples use the Soroban CLI. Replace `<CONTRACT_ID>` with your deployed contract address and `alice` with your configured identity.

### Initialize the Contract

Must be called once after deployment to set the initial admin.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS>
```

**Expected error if already initialized:**
```
Error: HostError: Value already exists
```

---

### Project Registry

#### Register a Project

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- register_project \
  --params '{
    "owner": "<OWNER_ADDRESS>",
    "name": "My DApp",
    "description": "A decentralized application on Stellar",
    "category": "DeFi",
    "website": "https://mydapp.example.com",
    "logo_cid": "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
    "metadata_cid": null
  }'
```

Returns the new project ID (e.g., `1`).

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectAlreadyExists` | A project with the same name is already registered |
| `InvalidProjectName` | Name is empty or whitespace only |
| `ProjectNameTooLong` | Name exceeds the maximum allowed length |
| `InvalidProjectDescription` | Description is empty or whitespace only |
| `ProjectDescriptionTooLong` | Description exceeds the maximum allowed length |
| `InvalidProjectCategory` | Category is empty or whitespace only |
| `InvalidProjectWebsite` | Website URL format is invalid |
| `MaxProjectsExceeded` | Global project limit has been reached |

#### Update a Project

Only the project owner can update. All fields are optional — only provided fields are changed.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- update_project \
  --params '{
    "project_id": 1,
    "caller": "<OWNER_ADDRESS>",
    "name": "My DApp v2",
    "description": "Updated description",
    "category": null,
    "website": null,
    "logo_cid": null,
    "metadata_cid": null
  }'
```

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotFound` | No project exists with the given ID |
| `Unauthorized` | Caller is not the project owner |

#### Get a Project

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_project \
  --project_id 1
```

Returns `null` if the project does not exist.

#### List Projects

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- list_projects \
  --start_id 1 \
  --limit 10
```

#### Get Projects by Owner

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_projects_by_owner \
  --owner <OWNER_ADDRESS>
```

#### Transfer Project Ownership

```bash
# Step 1: Initiate transfer (current owner)
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- initiate_transfer \
  --project_id 1 \
  --caller <CURRENT_OWNER_ADDRESS> \
  --new_owner <NEW_OWNER_ADDRESS>

# Step 2: Accept transfer (new owner)
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source new_owner_identity \
  --network testnet \
  -- accept_transfer \
  --project_id 1 \
  --caller <NEW_OWNER_ADDRESS>

# Cancel a pending transfer (current owner)
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- cancel_transfer \
  --project_id 1 \
  --caller <CURRENT_OWNER_ADDRESS>
```

**Common errors:**

| Error | Cause |
|---|---|
| `TransferNotFound` | No pending transfer exists for this project |
| `NotPendingTransferRecipient` | Caller is not the designated new owner |
| `Unauthorized` | Caller is not the current owner |

---

### Review System

#### Submit a Review

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source reviewer_identity \
  --network testnet \
  -- submit_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS> \
  --rating 5 \
  --review_cid "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
```

`rating` must be between 1 and 5. `review_cid` is an IPFS CID pointing to the review content.

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotFound` | No project exists with the given ID |
| `InvalidRating` | Rating is not between 1 and 5 |
| `DuplicateReview` | Reviewer has already submitted a review for this project |

#### Add a Review (legacy, optional CID)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source reviewer_identity \
  --network testnet \
  -- add_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS> \
  --rating 4 \
  --comment_cid '"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"'
```

#### Update a Review

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source reviewer_identity \
  --network testnet \
  -- update_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS> \
  --rating 3 \
  --comment_cid null
```

**Common errors:**

| Error | Cause |
|---|---|
| `ReviewNotFound` | No review exists for this project/reviewer pair |
| `NotReviewOwner` | Caller is not the original reviewer |

#### Delete a Review

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source reviewer_identity \
  --network testnet \
  -- delete_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS>
```

#### Get a Review

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS>
```

#### Respond to a Review (project owner)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- respond_to_review \
  --project_id 1 \
  --caller <OWNER_ADDRESS> \
  --reviewer <REVIEWER_ADDRESS> \
  --response "Thank you for your feedback!"
```

#### Get Project Stats

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_project_stats \
  --project_id 1
```

Returns `{ rating_sum, review_count, average_rating }`.

---

### Fee Management

#### Configure Fees (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- set_fee \
  --admin <ADMIN_ADDRESS> \
  --token '"<TOKEN_CONTRACT_ADDRESS>"' \
  --verification_fee 1000000 \
  --registration_fee 500000 \
  --treasury <TREASURY_ADDRESS>
```

Set `--token` to `null` to use the native XLM token.

**Common errors:**

| Error | Cause |
|---|---|
| `AdminOnly` | Caller is not an admin |

#### Pay a Fee

Must be called before requesting verification if a fee is configured.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source payer_identity \
  --network testnet \
  -- pay_fee \
  --payer <PAYER_ADDRESS> \
  --project_id 1 \
  --token '"<TOKEN_CONTRACT_ADDRESS>"'
```

**Common errors:**

| Error | Cause |
|---|---|
| `FeeConfigNotSet` | No fee configuration has been set |
| `TreasuryNotSet` | Treasury address is not configured |
| `InsufficientFee` | Transferred amount is less than the required fee |

#### Get Fee Configuration

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_fee_config
```

---

### Verification

#### Request Verification

The project owner submits evidence for admin review. If a verification fee is configured, `pay_fee` must be called first.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- request_verification \
  --project_id 1 \
  --requester <OWNER_ADDRESS> \
  --evidence_cid "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
```

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotFound` | No project exists with the given ID |
| `Unauthorized` | Caller is not the project owner |
| `InvalidStatusTransition` | Project is already pending or verified |

#### Approve Verification (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- approve_verification \
  --project_id 1 \
  --admin <ADMIN_ADDRESS>
```

**Common errors:**

| Error | Cause |
|---|---|
| `AdminOnly` | Caller is not an admin |
| `VerificationNotFound` | No verification request exists for this project |
| `InvalidStatusTransition` | Verification is not in `Pending` state |

#### Reject Verification (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- reject_verification \
  --project_id 1 \
  --admin <ADMIN_ADDRESS>
```

#### Revoke Verification (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- revoke_verification \
  --project_id 1 \
  --admin <ADMIN_ADDRESS> \
  --reason "Violated terms of service"
```

**Common errors:**

| Error | Cause |
|---|---|
| `VerificationNotRevocable` | Project is not currently in `Verified` state |

#### Get Verification Status

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_verification \
  --project_id 1
```

Returns a `VerificationRecord` with status `Unverified`, `Pending`, `Verified`, or `Rejected`.

**Common errors:**

| Error | Cause |
|---|---|
| `VerificationNotFound` | No verification record exists for this project |

---

### Admin Management

#### Add an Admin

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- add_admin \
  --caller <EXISTING_ADMIN_ADDRESS> \
  --new_admin <NEW_ADMIN_ADDRESS>
```

#### Remove an Admin

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- remove_admin \
  --caller <EXISTING_ADMIN_ADDRESS> \
  --admin_to_remove <ADMIN_ADDRESS_TO_REMOVE>
```

**Common errors:**

| Error | Cause |
|---|---|
| `CannotRemoveLastAdmin` | Removing this admin would leave the contract with no admins |
| `AdminNotFound` | The address to remove is not an admin |

#### Check Admin Status

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- is_admin \
  --address <ADDRESS>
```

---

## Features

- **Project Registry**: Register and manage project metadata on-chain
- **Review System**: Submit and manage project reviews with ratings
- **Verification**: Request and approve project verification
- **Fee Management**: Configurable fees for operations
- **Access Control**: Owner-based permissions
- **TTL Management**: Automatic and manual Time-To-Live extension for persistent storage

## TTL (Time To Live) Management

The contract implements comprehensive TTL management for Soroban persistent storage to ensure data doesn't expire unexpectedly.

### TTL Thresholds

- **Critical Data** (admin, fees, treasury): 30 days
- **Project Data**: 90 days
- **Review Data**: 60 days
- **Verification Data**: 45 days
- **User Data**: 60 days

### Manual TTL Extension Functions

```bash
# Extend TTL for a specific project
soroban contract invoke --id <CONTRACT_ID> --network testnet \
  -- extend_project_ttl --project_id 1

# Extend TTL for critical configuration
soroban contract invoke --id <CONTRACT_ID> --network testnet \
  -- extend_critical_config_ttl

# Extend TTL for user data
soroban contract invoke --id <CONTRACT_ID> --network testnet \
  -- extend_user_ttl --user <USER_ADDRESS>

# Extend TTL for a review
soroban contract invoke --id <CONTRACT_ID> --network testnet \
  -- extend_review_ttl --project_id 1 --reviewer <REVIEWER_ADDRESS>

# Extend TTL for verification data
soroban contract invoke --id <CONTRACT_ID> --network testnet \
  -- extend_verification_ttl --project_id 1
```

---

## Development

### Using Makefile

```bash
make help           # Show all commands
make build          # Build contract
make test           # Run tests
make test-verbose   # Run tests with output
make fmt            # Format code
make lint           # Run linter
make clean          # Clean artifacts
make dev            # Run full dev workflow (fmt + lint + test + build)
make ci             # Run CI checks (check + lint + test)
```

### Manual Commands

```bash
cargo build --target wasm32-unknown-unknown --release
cargo test
cargo fmt
cargo clippy
cargo clean
```

---

## Project Structure

```
src/
├── lib.rs                    # Main contract interface
├── constants.rs              # Constants, limits, and TTL thresholds
├── errors.rs                 # Error definitions
├── events.rs                 # Event emissions
├── fee_manager.rs            # Fee handling
├── project_registry.rs       # Project management
├── review_registry.rs        # Review system
├── verification_registry.rs  # Verification logic
├── rating_calculator.rs      # Rating calculations
├── storage_keys.rs           # Storage keys
├── storage_manager.rs        # TTL management
├── types.rs                  # Data structures
└── tests/                    # Tests
```

## Resources

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar Developer Portal](https://developers.stellar.org/)
- [Soroban Examples](https://github.com/stellar/soroban-examples)

## Contributing

Contributions are welcome! Please open an issue or pull request.
