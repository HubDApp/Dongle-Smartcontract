# Error Code Reference

All contract errors are defined in [`dongle-smartcontract/src/errors.rs`](dongle-smartcontract/src/errors.rs)
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
| 4 | `InvalidRating` | Rating value is outside the valid range (1–5). | Submit a rating between 1 and 5 inclusive. |
| 5 | `ReviewNotFound` | No review exists for this (project, reviewer) pair. | Confirm the project ID and reviewer address; the reviewer may not have submitted a review yet. |
| 6 | `DuplicateReview` | This reviewer has already submitted a review for the project. | Call `update_review` to modify the existing review instead. |
| 7 | `NotReviewOwner` | Caller is not the address that originally submitted the review. | Use the same address that created the review. |
| 8 | `VerificationNotFound` | No verification record exists for this project. | Call `request_verification` before querying or acting on verification state. |
| 9 | `InvalidStatus` | The requested verification status transition is not allowed by the state machine. | See valid transitions: `Unverified→Pending`, `Pending→Verified`, `Pending→Rejected`, `Verified→Unverified` (revoke). |
| 10 | `AdminOnly` | Action requires admin privileges. | Use an address that has been granted admin role via `add_admin`. |
| 11 | `FeeConfigNotSet` | No fee configuration exists. | Admin must call `set_fee` before fee-gated operations are available. |
| 12 | `TreasuryNotSet` | No treasury address is configured. | Admin must configure the treasury address as part of fee setup. |
| 13 | `InsufficientFee` | The fee payment amount is less than the configured minimum. | Pay at least the amount returned by the current fee configuration. |
| 14 | `InvalidProjectData` | One or more project fields failed validation (generic). | Check all field lengths and formats; see specific errors below for field-level detail. |
| 15 | `ProjectNameTooLong` | Project name exceeds the maximum allowed length. | Shorten the project name. |
| 16 | `InvalidNameFormat` | Project name contains invalid characters. | Use alphanumeric characters and allowed punctuation only. |
| 17 | `CannotRemoveLastAdmin` | Removing the only remaining admin is not permitted. | Add a second admin before removing the current one. |
| 18 | `ProjectTooYoung` | Project was registered too recently to request verification. | Wait until the project is older than the configured `min_project_age`. |
| 19 | `InvalidTag` | A tag value is empty or exceeds the maximum length. | Use non-empty tags within the character limit. |
| 20 | `TooManyTags` | Number of tags exceeds the per-project limit. | Remove tags until the count is within the allowed maximum. |
| 21 | `InvalidSocialLink` | A social link URL has an invalid format. | Provide a valid URL (e.g., `https://...`). |
| 22 | `TooManySocialLinks` | Number of social links exceeds the per-project limit. | Remove links until the count is within the allowed maximum. |
| 23 | `AlreadyReported` | Caller has already reported this project. | Each address can only report a project once. |
| 24 | `AdminNotFound` | The specified address does not hold an admin role. | Confirm the address has been added via `add_admin`. |
| 26 | `InvalidProjectName` | Project name fails format validation (distinct from length check). | Ensure the name meets character set requirements. |
| 27 | `InvalidProjectDesc` | Project description contains invalid characters or structure. | Check the description against allowed characters and encoding. |
| 28 | `InvalidCategory` | Category value is empty or otherwise malformed. | Provide a non-empty, properly formatted category string. |
| 29 | `ProjectDescTooLong` | Project description exceeds the maximum allowed length. | Shorten the description to fit within the character limit. |
| 30 | `MaxProjectsExceeded` | An owner or project has hit a registration or review limit. | Check `MAX_PROJECTS_PER_USER` and `MAX_REVIEWS_PER_PROJECT` constants in `constants.rs`. |
| 31 | `InvalidWebsite` | Website URL format is invalid. | Provide a valid URL starting with `https://`. |
| 32 | `InvalidLogoCid` | Logo CID is not a valid IPFS content identifier. | Provide a valid CIDv0 or CIDv1 string. |
| 33 | `InvalidMetaCid` | Metadata CID is not a valid IPFS content identifier. | Provide a valid CIDv0 or CIDv1 string. |
| 36 | `TransferNotFound` | No pending ownership transfer exists for this project. | Call `initiate_transfer` before attempting to accept or cancel. |
| 38 | `NotTransferRecip` | Caller is not the designated recipient of the pending transfer. | Use the address that was specified as `new_owner` when the transfer was initiated. |
| 39 | `ReviewsDisabled` | Reviews have been disabled for this project by the owner. | Contact the project owner; they can re-enable reviews via `set_reviews_enabled`. |
| 40 | `ReviewAlreadyReported` | Caller has already reported this review. | Each address can only report a given review once. |
| 41 | `ReviewAlreadyHidden` | Review is already in a hidden state. | No action needed; moderation already applied. |
| 42 | `ReviewNotHidden` | Attempted to restore a review that is not hidden. | Only hidden reviews can be restored via `restore_review`. |
| 43 | `AlreadyArchived` | Project is already archived. | Check project status before calling `archive_project`. |
| 44 | `ProjectNotArchived` | Project is not in an archived state. | Only archived projects can be reactivated via `reactivate_project`. |
| 45 | `ReportsCleared` | Report records were cleared automatically after the threshold was reached. | This is informational; the report count was reset after moderation action. |
| 46 | `CollectionNotFound` | The specified collection ID does not exist. | Verify the collection ID is correct and the collection has been created. |
| 47 | `CollectionExists` | A collection with this name already exists. | Use a unique collection name. |
| 48 | `AlreadyInCollection` | Project is already a member of this collection. | The project was previously added; no duplicate addition is allowed. |
| 49 | `AlreadyLinked` | The two projects are already linked to each other. | Unlink them first via `unlink_project` before re-linking. |
| 50 | `CannotLinkToSelf` | A project cannot be linked to itself. | Provide a different target project ID. |
| 51 | `AlreadyFollowing` | Caller is already following this project. | Unfollow first via the appropriate call before following again. |
| 52 | `NotFollowing` | Caller is not following this project. | Follow the project before attempting to unfollow. |
| 53 | `VerifiedFieldFrozen` | A metadata field is frozen and cannot be modified once the project is verified. | Revoke verification first (admin-only) if the field must change; prefer not changing verified metadata. |
| 54 | `NativeFeeNotSupported` | Native XLM fee payment is not supported. | Use the configured token contract address for fee payment. |

> **Gaps in numeric codes** (25, 34, 35, 37): These codes are intentionally
> unassigned. Do not use them for new errors to avoid ambiguity with any
> previously emitted values from older contract versions.

## Adding New Error Codes

1. Add the variant to `ContractError` in `errors.rs` with the next available number.
2. Add a row to the table above in the same PR.
3. Do not reuse a previously-assigned numeric value.
4. Update any relevant integration tests or frontend error-handling code.
