# Error Code Reference

All contract errors are defined in [`dongle-smartcontract/src/errors.rs`](../dongle-smartcontract/src/errors.rs)
as variants of `ContractError`. They are returned as Soroban `u32` error codes.

> **Maintenance checklist:** Whenever you add, remove, or rename a variant in
> `ContractError`, update this table in the same PR. The numeric value is part
> of the on-chain ABI and must never be reused for a different meaning once
> deployed.

## Error Table

| Code | Name | Meaning | Likely Fix |
|------|------|---------|-----------|
| 1 | `ProjectNotFound` | The requested project ID does not exist in storage. | Verify the project ID is correct and the project has been registered. |
| 2 | `Unauthorized` | Caller does not have permission to perform this action. | Ensure the caller is the project owner, an admin, or otherwise authorized. |
| 3 | `ProjectAlreadyExists` | A project with this name or slug is already registered. | Choose a unique project name and slug. |
| 4 | `InvalidRating` | Rating value is outside the valid range (1-5). | Submit a rating between 1 and 5 inclusive. |
| 5 | `ReviewNotFound` | No review exists for this (project, reviewer) pair. | Confirm the project ID and reviewer address; the reviewer may not have submitted a review yet. |
| 6 | `DuplicateReview` | This reviewer has already submitted a review for the project. | Call `update_review` to modify the existing review instead. |
| 7 | `NotReviewOwner` | Caller is not the address that originally submitted the review. | Use the same address that created the review. |
| 8 | `VerificationNotFound` | No verification record exists for this project. | Call `request_verification` before querying or acting on verification state. |
| 9 | `InvalidStatusTransition` | The requested verification status transition is not allowed by the state machine. | Check the current verification status before transitioning; not every state can move to every other state. |
| 10 | `AdminOnly` | Only admin can perform this action. | Use an address that has been granted admin role via `add_admin`. |
| 11 | `FeeConfigNotSet` | No fee configuration exists. | Admin must call `set_fee` before fee-gated operations are available. |
| 12 | `TreasuryNotSet` | No treasury address is configured. | Admin must configure the treasury address as part of fee setup. |
| 13 | `InsufficientFee` | The fee payment amount is less than the configured minimum. | Pay at least the amount returned by the current fee configuration. |
| 14 | `InvalidProjectData` | One or more project fields failed validation (generic / missing required fields). | Check all field lengths and formats; see specific errors below for field-level detail. |
| 15 | `ProjectNameTooLong` | Project name exceeds the maximum allowed length. | Shorten the project name. |
| 16 | `InvalidProjectNameFormat` | Project name contains invalid characters. | Use alphanumeric characters and allowed punctuation only. |
| 17 | `CannotRemoveLastAdmin` | Removing the only remaining admin is not permitted. | Add a second admin before removing the current one. |
| 18 | `AdminNotFound` | The specified address does not hold an admin role. | Confirm the address has been added via `add_admin`. |
| 19 | `InvalidProjectName` | Project name is empty or whitespace only. | Provide a non-empty project name. |
| 20 | `InvalidProjectDescription` | Project description is empty or whitespace only. | Provide a non-empty project description. |
| 21 | `InvalidProjectCategory` | Project category is empty or whitespace only. | Provide a non-empty, properly formatted category string. |
| 22 | `ProjectDescriptionTooLong` | Project description exceeds the maximum allowed length. | Shorten the description to fit within the character limit. |
| 23 | `InvalidProjectDescriptionFormat` | Project description contains invalid characters. | Check the description against allowed characters and encoding. |
| 24 | `MaxProjectsExceeded` | An owner has hit the per-owner project registration limit. | Check the `MAX_PROJECTS_PER_USER` constant in `constants.rs`, or transfer/archive an existing project. |
| 25 | `InvalidProjectWebsite` | Website URL format is invalid. | Provide a valid URL starting with `https://`. |
| 26 | `InvalidProjectLogoCid` | Logo CID is not a valid IPFS content identifier. | Provide a valid CIDv0 or CIDv1 string. |
| 27 | `InvalidProjectMetadataCid` | Metadata CID is not a valid IPFS content identifier. | Provide a valid CIDv0 or CIDv1 string. |
| 28 | `ProjectCategoryTooLong` | Project category exceeds the maximum allowed length. | Shorten the category string. |
| 29 | `ProjectWebsiteTooLong` | Project website exceeds the maximum allowed length. | Shorten the website URL. |
| 30 | `VerificationNotRevocable` | Project is not in a revocable state (must be `Verified`). | Only call `revoke_verification` on projects whose status is `Verified`. |
| 31 | `TransferNotFound` | No pending ownership transfer exists for this project. | Call `initiate_transfer` before attempting to accept or cancel. |
| 32 | `NotPendingTransferRecipient` | Caller is not the designated recipient of the pending transfer. | Use the address that was specified as `new_owner` when the transfer was initiated. |
| 33 | `VerificationExpired` | Verification has expired and is no longer active. | Request a fresh verification via `request_verification`. |
| 34 | `AlreadyInitialized` | Contract has already been initialized. | `initialize` can only be called once per deployed contract instance. |
| 35 | `InvalidCid` | Invalid CID format. | Provide a valid CIDv0 or CIDv1 string. |
| 36 | `InvalidInput` | Invalid input provided (generic). | Check the request parameters against the method's documented constraints. |
| 37 | `InvalidProjectSlug` | Project slug fails format or length validation. | Use a URL-safe, lowercase slug within the allowed length. |
| 38 | `InvalidStatus` | Invalid status for the requested operation. | Check the entity's current status before performing this action. |
| 39 | `AlreadyArchived` | Project is already archived. | Check project status before calling `archive_project`. |
| 40 | `ProjectNotArchived` | Project is not in an archived state. | Only archived projects can be reactivated via `reactivate_project`. |
| 41 | `ProjectTooYoung` | Project was registered too recently to request verification. | Wait until the project is older than the configured `MIN_PROJECT_AGE_SECONDS`. |
| 42 | `VerifiedFieldFrozen` | A metadata field is frozen and cannot be modified once the project is verified. | Revoke verification first (admin-only) if the field must change; prefer not changing verified metadata. |
| 43 | `ReservedName` | Project name is reserved and cannot be used. | Choose a different, non-reserved project name. |
| 44 | `DuplicateProjectName` | A project with this name already exists. | Choose a unique project name. |
| 45 | `CannotLinkToSelf` | A project cannot be linked to itself. | Provide a different target project ID. |
| 46 | `AlreadyLinked` | The two projects are already linked to each other. | Unlink them first via `unlink_project` before re-linking. |
| 47 | `AlreadyFollowing` | Caller is already following this project. | Unfollow first via the appropriate call before following again. |
| 48 | `NotFollowing` | Caller is not following this project. | Follow the project before attempting to unfollow. |
| 49 | `AlreadyReported` | Review has already been reported by this caller. | Each address can only report a given review once. |
| 50 | `ReviewAlreadyHidden` | Review is already in a hidden state. | No action needed; moderation already applied. |
| 51 | `ReviewNotHidden` | Attempted to restore a review that is not hidden. | Only hidden reviews can be restored via `restore_review`. |
| 52 | `CollectionNotFound` | The specified collection ID does not exist. | Verify the collection ID is correct and the collection has been created. |
| 53 | `CollectionExists` | A collection with this name already exists. | Use a unique collection name. |
| 54 | `AlreadyInCollection` | Project is already a member of this collection. | The project was previously added; no duplicate addition is allowed. |
| 55 | `ReviewsDisabled` | Reviews have been disabled for this project by the owner. | Contact the project owner; they can re-enable reviews via `set_reviews_enabled`. |
| 56 | `OwnerCannotReview` | Project owner cannot review their own project. | Use a different address to submit the review. |
| 57 | `InvalidNameFormat` | Name contains invalid characters or formatting. | Use alphanumeric characters and allowed punctuation only. |
| 58 | `ReviewerNotEligible` | Reviewer does not meet the eligibility requirements to review this project. | Confirm the reviewer meets the project's review eligibility rules before submitting. |
| 59 | `ReviewFeeRequired` | A review fee is required but was not paid. | Pay at least the configured review fee before submitting the review. |
| 60 | `CollectionFull` | Collection has reached the maximum number of projects. | Check the `MAX_PROJECTS_PER_COLLECTION` constant, or remove a project before adding another. |
| 61 | `ContractPaused` | Contract is paused. | Wait until an admin lifts the emergency pause via `unpause`. |
| 62 | `AlreadyBookmarked` | Project is already bookmarked by the caller. | Each address can only bookmark a project once. |
| 63 | `AlreadyEndorsed` | Project is already endorsed by the caller. | Each address can only endorse a project once. |
| 64 | `NotBookmarked` | Project is not bookmarked by the caller. | Bookmark the project before attempting to unbookmark it. |
| 65 | `NotEndorsed` | Project is not endorsed by the caller. | Endorse the project before attempting to remove the endorsement. |
| 66 | `TimelockNotExpired` | Timelock action has not expired yet. | Wait until the timelock delay has elapsed before executing the action. |
| 67 | `PayloadHashMismatch` | Stored proposal payload does not match its recorded hash. | Re-submit the proposal; the payload used to execute it must match the one that was proposed. |
| 68 | `InvalidTags` | Tag list is invalid: empty tag, over-length tag, too many tags, invalid characters, or duplicate values (case-insensitive after ASCII-lowercase normalization). | Provide unique, non-empty tags within `MAX_TAGS_PER_PROJECT` / `MAX_TAG_LENGTH` using only `[A-Za-z0-9_-]`. |
| 69 | `ProposalExpired` | Admin proposal has passed its expiry time and can no longer be executed. | Create a new proposal; expired proposals cannot be executed. |
| 70 | `NoRefundAvailable` | No refund is recorded for the given project. | Confirm the verification request was actually rejected with a fee refund recorded. |
| 71 | `RefundAlreadyClaimed` | The recorded refund has already been paid out. | No action needed; the refund was already claimed. |
| 72 | `ArithmeticOverflow` | A checked arithmetic operation overflowed. | Reduce the magnitude of the input values; this indicates an unexpectedly large accumulated amount or count. |
| 73 | `NotInCollection` | Project is not a member of this collection. | Add the project to the collection via `add_project_to_collection` before attempting to remove it. |
| 74 | `ThresholdDowngradeRequiresSupermajority` | A `SetThreshold` proposal that would lower the current approval threshold does not have enough approvals. The number of approvals must be **strictly greater than** the proposed new threshold (supermajority rule). | Gather additional admin approvals before executing the downgrade proposal. The required count is `new_threshold + 1`. |

## Adding New Error Codes

1. Add the variant to `ContractError` in `errors.rs` with the next available number.
2. Add a row to the table above in the same PR.
3. Do not reuse a previously-assigned numeric value.
4. Update any relevant integration tests or frontend error-handling code.
