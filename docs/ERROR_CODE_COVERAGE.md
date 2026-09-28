# Error Code Coverage Matrix

This document tracks, for every variant of `ContractError`
([`errors.rs`](../dongle-smartcontract/src/errors.rs)), whether the code is
**returned by production contract logic** and whether at least one test
**asserts** that error path.

- **Returned by** – the module(s) whose non-test code path constructs the
  variant (`Err(ContractError::X)` / `.into()` / `require`-style helpers).
  `—` means no production code path currently returns it.
- **Tested** – at least one file under `dongle-smartcontract/src/tests/`
  asserts the variant (typically `assert_eq!(err, ContractError::X)` on a
  `try_*` call).
- **Status**:
  - `live` – returned by production code **and** covered by a test.
  - `live · untested` – returned by production code, no dedicated test asserting it.
  - `reserved` – **not** returned by any production code path. Kept as a stable
    ABI slot; see [Reserved codes](#reserved-codes) for the rationale.

> The numeric value of every variant is part of the on-chain ABI and must never
> be reused for a different meaning. Reserved variants are therefore **retained,
> not deleted** — removal is only safe in a coordinated ABI break.

## Matrix

| Code | Name | Returned by | Tested | Status |
|------|------|-------------|--------|--------|
| 1 | `ProjectNotFound` | project/review/verification/fee/report/endorsement registries | yes | live |
| 2 | `Unauthorized` | auth, admin_manager, all registries | yes | live |
| 3 | `ProjectAlreadyExists` | project_registry | yes | live |
| 4 | `InvalidRating` | review_registry::validation | yes | live |
| 5 | `ReviewNotFound` | review_registry::storage, changelog_registry | yes | live |
| 6 | `DuplicateReview` | review_registry::storage | yes | live |
| 7 | `NotReviewOwner` | review_registry::storage | no | live · untested |
| 8 | `VerificationNotFound` | verification_registry::storage, admin_manager | yes | live |
| 9 | `InvalidStatusTransition` | verification_registry::state_machine | no | live · untested |
| 10 | `AdminOnly` | admin_manager, dispute/report registries | yes | live |
| 11 | `FeeConfigNotSet` | fee_manager, lib | yes | live |
| 12 | `TreasuryNotSet` | fee_manager | yes | live |
| 13 | `InsufficientFee` | fee_manager | yes | live |
| 14 | `InvalidProjectData` | utils validation, project_registry | yes | live |
| 15 | `ProjectNameTooLong` | — | — | reserved |
| 16 | `InvalidProjectNameFormat` | — | — | reserved |
| 17 | `CannotRemoveLastAdmin` | admin_manager | no | live · untested |
| 18 | `AdminNotFound` | admin_manager, project_registry | yes | live |
| 19 | `InvalidProjectName` | utils::validate_project_name, project_registry | yes | live |
| 20 | `InvalidProjectDescription` | — | — | reserved |
| 21 | `InvalidProjectCategory` | — | — | reserved |
| 22 | `ProjectDescriptionTooLong` | — | — | reserved |
| 23 | `InvalidProjectDescriptionFormat` | — | — | reserved |
| 24 | `MaxProjectsExceeded` | project_registry, collection_registry, review indexes | yes | live |
| 25 | `InvalidProjectWebsite` | — | — | reserved |
| 26 | `InvalidProjectLogoCid` | — | — | reserved |
| 27 | `InvalidProjectMetadataCid` | — | — | reserved |
| 28 | `ProjectCategoryTooLong` | — | — | reserved |
| 29 | `ProjectWebsiteTooLong` | — | — | reserved |
| 30 | `VerificationNotRevocable` | — | — | reserved |
| 31 | `TransferNotFound` | project_registry | yes | live |
| 32 | `NotPendingTransferRecipient` | — | — | reserved |
| 33 | `VerificationExpired` | — | — | reserved |
| 34 | `AlreadyInitialized` | admin_manager | yes | live |
| 35 | `InvalidCid` | utils, changelog_registry, dependency_registry | yes | live |
| 36 | `InvalidInput` | utils (category/website/CID validators), timelock_manager, dependency_registry | yes | live |
| 37 | `InvalidProjectSlug` | utils::validate_project_slug | yes | live |
| 38 | `InvalidStatus` | verification/fee/report/admin state guards | yes | live |
| 39 | `AlreadyArchived` | project_registry, dispute_registry | yes | live |
| 40 | `ProjectNotArchived` | project_registry | yes | live |
| 41 | `ProjectTooYoung` | verification_registry::storage | yes | live |
| 42 | `VerifiedFieldFrozen` | utils / project_registry freeze guard | yes | live |
| 43 | `ReservedName` | project_registry | yes | live |
| 44 | `DuplicateProjectName` | project_registry | no | live · untested |
| 45 | `CannotLinkToSelf` | project_registry, dispute_registry | yes | live |
| 46 | `AlreadyLinked` | project_registry, dependency_registry | yes | live |
| 47 | `AlreadyFollowing` | subscription_registry | no | live · untested |
| 48 | `NotFollowing` | subscription_registry | no | live · untested |
| 49 | `AlreadyReported` | report_registry, review_registry::storage | yes | live |
| 50 | `ReviewAlreadyHidden` | review_registry::storage | yes | live |
| 51 | `ReviewNotHidden` | review_registry::storage | yes | live |
| 52 | `CollectionNotFound` | collection_registry | yes | live |
| 53 | `CollectionExists` | collection_registry | yes | live |
| 54 | `AlreadyInCollection` | collection_registry | yes | live |
| 55 | `ReviewsDisabled` | review_registry::storage | yes | live |
| 56 | `OwnerCannotReview` | review_registry::validation | yes | live |
| 57 | `InvalidNameFormat` | — | — | reserved |
| 58 | `ReviewerNotEligible` | review_registry::storage | yes | live |
| 59 | `ReviewFeeRequired` | review_registry::storage | yes | live |
| 60 | `CollectionFull` | collection_registry | no | live · untested |
| 61 | `ContractPaused` | emergency_pause, config_registry, storage guards | yes | live |
| 62 | `AlreadyBookmarked` | bookmark_registry | no | live · untested |
| 63 | `AlreadyEndorsed` | endorsement_registry | yes | live |
| 64 | `NotBookmarked` | bookmark_registry | no | live · untested |
| 65 | `NotEndorsed` | endorsement_registry | yes | live |
| 66 | `TimelockNotExpired` | timelock_manager | yes | live |
| 67 | `PayloadHashMismatch` | admin_manager | yes | live |
| 68 | `InvalidTags` | utils tag validation | yes | live |
| 69 | `ProposalExpired` | admin_manager | yes | live |
| 70 | `NoRefundAvailable` | fee_manager | no | live · untested |
| 71 | `RefundAlreadyClaimed` | fee_manager | no | live · untested |
| 72 | `ArithmeticOverflow` | fee_manager (checked math) | no | live · untested |
| 73 | `NotInCollection` | collection_registry | yes | live |
| 74 | `ThresholdDowngradeRequiresSupermajority` | admin_manager | yes | live |

## Summary

| Status | Count |
|--------|-------|
| `live` | 47 |
| `live · untested` | 12 |
| `reserved` | 15 |
| **Total** | **74** |

## Reserved codes

The following 15 variants are **not** returned by any production code path. The
field-level validators in [`utils.rs`](../dongle-smartcontract/src/utils.rs)
were consolidated over time to return a small set of generic errors
(`InvalidProjectName`, `InvalidProjectData`, `InvalidInput`) instead of one
error per field/failure-mode. The finer-grained variants were kept as stable
ABI slots rather than removed.

| Code | Name | Superseded by | Rationale |
|------|------|---------------|-----------|
| 15 | `ProjectNameTooLong` | `InvalidProjectName` (19) | Name length is checked in `validate_project_name`, which returns the single generic name error. |
| 16 | `InvalidProjectNameFormat` | `InvalidProjectName` (19) | Character-set validation folded into `validate_project_name`. |
| 20 | `InvalidProjectDescription` | `InvalidProjectData` (14) | `validate_description` returns the generic data error for empty/whitespace descriptions. |
| 21 | `InvalidProjectCategory` | `InvalidInput` (36) | `validate_category_field` returns `InvalidInput`. |
| 22 | `ProjectDescriptionTooLong` | `InvalidProjectData` (14) | Length branch of `validate_description`. |
| 23 | `InvalidProjectDescriptionFormat` | `InvalidProjectData` (14) | No separate character-set check is performed on descriptions. |
| 25 | `InvalidProjectWebsite` | `InvalidInput` (36) | `validate_website` returns `InvalidInput` for bad scheme/length. |
| 26 | `InvalidProjectLogoCid` | `InvalidCid` (35) | Logo CIDs run through the shared `validate_cid` helper. |
| 27 | `InvalidProjectMetadataCid` | `InvalidCid` (35) | Metadata CIDs run through the shared `validate_cid` helper. |
| 28 | `ProjectCategoryTooLong` | `InvalidInput` (36) | Length branch of `validate_category_field`. |
| 29 | `ProjectWebsiteTooLong` | `InvalidInput` (36) | Length branch of `validate_website`. |
| 30 | `VerificationNotRevocable` | `InvalidStatus` (38) | The revoke path guards on status and returns `InvalidStatus`. |
| 32 | `NotPendingTransferRecipient` | `Unauthorized` (2) | `accept_transfer` / `cancel_transfer` authorise the caller and return `Unauthorized` on mismatch. |
| 33 | `VerificationExpired` | `InvalidStatus` (38) / `is_verification_active` | Expiry is surfaced as a boolean (`is_verification_active`, `is_verification_expired`) and as `InvalidStatus` on state-gated calls; a `VerificationExpiredEvent` is also emitted. |
| 57 | `InvalidNameFormat` | `InvalidProjectName` (19) / `InvalidInput` (36) | Generic name/format failures use the existing generic errors. |

### Options for reserved codes

1. **Keep as reserved (current choice).** Zero ABI risk. This document is the
   "documented exception" required by the coverage policy.
2. **Wire them into the validators** so each failure mode returns its specific
   code (e.g. `validate_project_name` returns `ProjectNameTooLong` on the length
   branch). This improves error ergonomics for integrators but changes observable
   behaviour and every test/asserting client that currently expects the generic
   code — do it in a dedicated, versioned change.
3. **Remove in a coordinated ABI break.** Only acceptable alongside a contract
   version bump and a migration note for indexers/front-ends.

## Maintenance

When you add, remove, or change a `ContractError` variant:

1. Update [`ERROR_CODES.md`](./ERROR_CODES.md) (the reference table).
2. Update the matrix above (row + summary counts).
3. If the variant is newly returned by code, add or note the test that asserts it.
4. Never reuse a numeric value.
