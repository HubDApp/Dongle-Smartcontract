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

#### Get Verification History

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_verification_history \
  --project_id 1
```

Returns a vector of all verification records for the project, including historical status changes.

#### Request Verification Renewal

When a verified project's verification is expiring, the owner can request renewal.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- request_renewal \
  --project_id 1 \
  --requester <OWNER_ADDRESS> \
  --evidence_cid "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
```

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotFound` | No project exists with the given ID |
| `Unauthorized` | Caller is not the project owner |
| `InvalidStatusTransition` | Project is not currently verified |

#### Approve Renewal (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- approve_renewal \
  --project_id 1 \
  --admin <ADMIN_ADDRESS>
```

#### Reject Renewal (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- reject_renewal \
  --project_id 1 \
  --admin <ADMIN_ADDRESS>
```

#### Get Renewal Request

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_renewal_request \
  --project_id 1
```

#### Get Renewal History

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_renewal_history \
  --project_id 1 \
  --start_index 0 \
  --limit 10
```

#### Check if Verification is Expired

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- is_verification_expired \
  --project_id 1
```

Returns `true` if the verification has expired, `false` otherwise.

---

### Verification Configuration (admin only)

#### Set Minimum Project Age

Set the minimum age (in seconds) a project must exist before it can be verified.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- set_min_project_age \
  --admin <ADMIN_ADDRESS> \
  --min_age_seconds 86400
```

#### Get Minimum Project Age

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_min_project_age
```

#### Set Verification Duration

Set how long (in seconds) a verification remains valid before renewal is required.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- set_verification_duration \
  --admin <ADMIN_ADDRESS> \
  --duration_seconds 2592000
```

#### Get Verification Duration

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_verification_duration
```

---

### Project Linking

#### Link Projects

Link two related projects together (e.g., main project and its mobile app).

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- link_project \
  --project_id 1 \
  --caller <OWNER_ADDRESS> \
  --linked_project_id 2
```

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotFound` | One or both projects do not exist |
| `Unauthorized` | Caller is not the owner of the project |
| `AlreadyLinked` | Projects are already linked |
| `CannotLinkToSelf` | Cannot link a project to itself |

#### Unlink Projects

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- unlink_project \
  --project_id 1 \
  --caller <OWNER_ADDRESS> \
  --linked_project_id 2
```

#### Get Linked Projects

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_linked_projects \
  --project_id 1
```

Returns a vector of linked project IDs.

---

### Featured Projects (admin only)

#### Set Featured

Mark a project as featured or remove featured status.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- set_featured \
  --admin <ADMIN_ADDRESS> \
  --project_id 1 \
  --featured true
```

#### List Featured Projects

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- list_featured_projects \
  --start 0 \
  --limit 10
```

---

### Project Reporting

#### Report a Project

Report a project for spam, scams, broken links, or abusive metadata.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source reporter_identity \
  --network testnet \
  -- report_project \
  --project_id 1 \
  --reporter <REPORTER_ADDRESS> \
  --reason_cid "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
```

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotFound` | No project exists with the given ID |
| `AlreadyReported` | User has already reported this project |

#### Get Project Reports

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_project_reports \
  --project_id 1
```

#### Get Project Report Count

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_project_report_count \
  --project_id 1
```

#### Check if User Reported

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- has_user_reported \
  --project_id 1 \
  --reporter <REPORTER_ADDRESS>
```

#### Clear Project Reports (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- clear_project_reports \
  --project_id 1 \
  --admin <ADMIN_ADDRESS>
```

---

### Review Moderation

#### Report a Review

Report a review for moderation.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source reporter_identity \
  --network testnet \
  -- report_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS> \
  --reporter <REPORTER_ADDRESS>
```

**Common errors:**

| Error | Cause |
|---|---|
| `ReviewNotFound` | No review exists for this project/reviewer pair |
| `ReviewAlreadyReported` | This review has already been reported |

#### Hide a Review (admin only)

Hide a reported review from public view.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- hide_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS> \
  --admin <ADMIN_ADDRESS>
```

**Common errors:**

| Error | Cause |
|---|---|
| `ReviewAlreadyHidden` | Review is already hidden |

#### Restore a Review (admin only)

Restore a previously hidden review.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- restore_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS> \
  --admin <ADMIN_ADDRESS>
```

**Common errors:**

| Error | Cause |
|---|---|
| `ReviewNotHidden` | Review is not currently hidden |

#### Admin Delete Review (admin only)

Permanently delete a review (hard delete).

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- admin_delete_review \
  --project_id 1 \
  --reviewer <REVIEWER_ADDRESS> \
  --admin <ADMIN_ADDRESS>
```

#### Enable/Disable Reviews (project owner)

Enable or disable reviews for a project.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- set_reviews_enabled \
  --project_id 1 \
  --caller <OWNER_ADDRESS> \
  --enabled false
```

**Common errors:**

| Error | Cause |
|---|---|
| `ReviewsDisabled` | Reviews are disabled for this project |

#### Check if Reviews Enabled

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_reviews_enabled \
  --project_id 1
```

---

### Project Archiving

#### Archive a Project

Archive a project (owner only). Archived projects are not listed in general queries.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- archive_project \
  --project_id 1 \
  --caller <OWNER_ADDRESS>
```

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotFound` | No project exists with the given ID |
| `Unauthorized` | Caller is not the project owner |
| `AlreadyArchived` | Project is already archived |

#### Reactivate a Project

Reactivate an archived project (owner only).

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- reactivate_project \
  --project_id 1 \
  --caller <OWNER_ADDRESS>
```

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotArchived` | Project is not currently archived |

---

### Collections (admin only)

#### Create a Collection

Create a new curated collection of projects.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- create_collection \
  --admin <ADMIN_ADDRESS> \
  --name "DeFi Projects" \
  --description "Curated list of DeFi applications"
```

Returns the new collection ID.

**Common errors:**

| Error | Cause |
|---|---|
| `CollectionExists` | A collection with this name already exists |

#### Update a Collection

Update a collection's name and description.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- update_collection \
  --admin <ADMIN_ADDRESS> \
  --collection_id 1 \
  --name "Updated Collection Name" \
  --description "Updated description"
```

**Common errors:**

| Error | Cause |
|---|---|
| `CollectionNotFound` | No collection exists with the given ID |

#### Delete a Collection

Delete a collection and its project associations.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- delete_collection \
  --admin <ADMIN_ADDRESS> \
  --collection_id 1
```

#### Add Project to Collection

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- add_project_to_collection \
  --admin <ADMIN_ADDRESS> \
  --collection_id 1 \
  --project_id 1
```

**Common errors:**

| Error | Cause |
|---|---|
| `AlreadyInCollection` | Project is already in this collection |

#### Remove Project from Collection

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- remove_project_from_collection \
  --admin <ADMIN_ADDRESS> \
  --collection_id 1 \
  --project_id 1
```

#### Get a Collection

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_collection \
  --collection_id 1
```

#### List Collections

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- list_collections \
  --start 0 \
  --limit 10
```

#### List Collection Projects

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- list_collection_projects \
  --collection_id 1 \
  --start 0 \
  --limit 10
```

#### Get Collection Project Count

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_collection_project_count \
  --collection_id 1
```

#### Get Collection Count

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_collection_count
```

---

### Admin Action Log

#### Get Admin Action Log Entry

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_admin_action_log_entry \
  --log_id 1
```

#### List Admin Actions

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- list_admin_actions \
  --start 0 \
  --limit 10
```

Returns admin action log entries with pagination (most recent first).

#### Get Admin Action Log Count

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_admin_action_log_count
```

---

### Project Claiming

#### Set Project Claimable

Mark a project as claimable by others (owner only).

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- set_project_claimable \
  --project_id 1 \
  --caller <OWNER_ADDRESS> \
  --claimable true
```

#### Submit Claim Request

Submit a request to claim ownership of a claimable project.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source claimant_identity \
  --network testnet \
  -- submit_claim_request \
  --project_id 1 \
  --claimant <CLAIMANT_ADDRESS> \
  --proof_cid "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
```

Returns the claim request ID.

#### Approve Claim Request (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- approve_claim_request \
  --claim_request_id 1 \
  --admin <ADMIN_ADDRESS>
```

#### Reject Claim Request (admin only)

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- reject_claim_request \
  --claim_request_id 1 \
  --admin <ADMIN_ADDRESS>
```

#### Get Claim Request

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_claim_request \
  --claim_request_id 1
```

#### Get Claim Requests for Project

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_claim_requests_for_project \
  --project_id 1
```

---

### Project Dependencies

#### Add Project Dependency

Add a dependency to a project (owner only).

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- add_project_dependency \
  --project_id 1 \
  --caller <OWNER_ADDRESS> \
  --dependency '{
    "name": "Stellar SDK",
    "version": "2.0.0",
    "type": "library",
    "url": "https://github.com/stellar/js-stellar-sdk"
  }'
```

**Common errors:**

| Error | Cause |
|---|---|
| `ProjectNotFound` | No project exists with the given ID |
| `Unauthorized` | Caller is not the project owner |

#### Update Project Dependency

Update an existing dependency (owner only).

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- update_project_dependency \
  --project_id 1 \
  --caller <OWNER_ADDRESS> \
  --dependency_key '{
    "name": "Stellar SDK",
    "version": "2.0.0"
  }' \
  --new_dependency '{
    "name": "Stellar SDK",
    "version": "2.1.0",
    "type": "library",
    "url": "https://github.com/stellar/js-stellar-sdk"
  }'
```

#### Remove Project Dependency

Remove a dependency from a project (owner only).

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- remove_project_dependency \
  --project_id 1 \
  --caller <OWNER_ADDRESS> \
  --dependency_key '{
    "name": "Stellar SDK",
    "version": "2.0.0"
  }'
```

#### Get Project Dependencies

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_project_dependencies \
  --project_id 1
```

---

### Duplicate Disputes

#### Open Duplicate Dispute

Report a project as a duplicate of another project.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source reporter_identity \
  --network testnet \
  -- open_duplicate_dispute \
  --project_id 2 \
  --original_project_id 1 \
  --creator <REPORTER_ADDRESS> \
  --evidence_cid "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
```

Returns the dispute ID.

#### Resolve Duplicate Dispute (admin only)

Resolve a duplicate dispute with an action (keep_original, keep_duplicate, or merge).

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- resolve_duplicate_dispute \
  --dispute_id 1 \
  --admin <ADMIN_ADDRESS> \
  --action "KeepOriginal"
```

#### Get Duplicate Dispute

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_duplicate_dispute \
  --dispute_id 1
```

#### Get Disputes for Project

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_disputes_for_project \
  --project_id 1
```

---

### Advanced Query Functions

#### List Projects by Status

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- list_projects_by_status \
  --status "Verified" \
  --start_id 1 \
  --limit 10
```

#### List Projects by Category

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- list_projects_by_category \
  --category "DeFi" \
  --start_id 0 \
  --limit 10
```

#### List Projects by Tag

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- list_projects_by_tag \
  --tag "nft" \
  --start_id 0 \
  --limit 10
```

#### Get Projects by IDs

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_projects_by_ids \
  --ids '[1, 2, 3]'
```

#### Get Project by Slug

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_project_by_slug \
  --slug "my-dapp"
```

#### Get Stats Batch

Get statistics for multiple projects in a single call.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_stats_batch \
  --ids '[1, 2, 3]'
```

#### Get Verifications Batch

Get verification records for multiple projects in a single call.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_verifications_batch \
  --ids '[1, 2, 3]'
```

#### Get Reviews by IDs

Get multiple reviews in a single call.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_reviews_by_ids \
  --ids '[(1, "<REVIEWER_ADDRESS1>"), (2, "<REVIEWER_ADDRESS2>")]'
```

#### Get Project Review CIDs

Get all review CIDs for a project.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_project_review_cids \
  --project_id 1
```

#### Get Owner Project Count

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_owner_project_count \
  --owner <OWNER_ADDRESS>
```

#### Get Project Count

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_project_count
```

#### Get Admin Count

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_admin_count
```

#### Get Admin List

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_admin_list
```

---

### Verification History Management (admin only)

#### Clear Verification History

Prune verification history, keeping only the most recent `keep_count` records.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- clear_verification_history \
  --project_id 1 \
  --admin <ADMIN_ADDRESS> \
  --keep_count 5
```

Returns the number of records removed.

#### Clear Renewal History

Clear all renewal history records for a project.

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source admin_identity \
  --network testnet \
  -- clear_renewal_history \
  --project_id 1 \
  --admin <ADMIN_ADDRESS>
```

Returns the number of records removed.

---

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
