# Error Catalog

Every `ContractError` code with root cause, debugging steps, resolution, and a triggering example.

Use `ContractError::troubleshooting_hint()` on-chain for a short hint; this file is the full guide.
Searchable web reference: `docs/error-reference.html`.

## Index

| Code | Error | Summary |
| --- | --- | --- |
| 1 | `AlreadyInitialized` | Contract has already been initialized |
| 2 | `Unauthorized` | Unauthorized access - caller is not permitted |
| 3 | `ProjectNotFound` | Project not found |
| 4 | `InvalidRating` | Invalid rating - must be between 1 and 5 |
| 5 | `ReviewNotFound` | Review not found |
| 6 | `DuplicateReview` | Duplicate review submission for same project and reviewer |
| 7 | `NotReviewOwner` | Caller is not the owner of the targeted review |
| 8 | `VerificationNotFound` | Verification record not found |
| 9 | `InvalidStatusTransition` | Invalid verification status transition |
| 10 | `AdminOnly` | Only admin can perform this action |
| 11 | `FeeConfigNotSet` | Fee configuration not set |
| 12 | `TreasuryNotSet` | Treasury address not set |
| 13 | `InsufficientFee` | Insufficient fee paid |
| 14 | `InvalidProjectData` | Invalid project data - missing required fields |
| 15 | `ProjectNameTooLong` | Project name too long |
| 16 | `InvalidProjectNameFormat` | Invalid project name format |
| 17 | `CannotRemoveLastAdmin` | Cannot remove last admin |
| 18 | `AdminNotFound` | Admin not found |
| 19 | `InvalidProjectName` | Invalid project name - empty or whitespace only |
| 20 | `InvalidProjectDescription` | Invalid project description - empty or whitespace only |
| 21 | `InvalidProjectCategory` | Invalid project category - empty or whitespace only |
| 22 | `ProjectDescriptionTooLong` | Project description too long |
| 23 | `InvalidProjectDescriptionFormat` | Project description contains invalid characters |
| 24 | `MaxProjectsExceeded` | Maximum number of projects exceeded |
| 25 | `InvalidProjectWebsite` | Invalid project website |
| 26 | `InvalidProjectLogoCid` | Invalid project logo CID |
| 27 | `InvalidProjectMetadataCid` | Invalid project metadata CID |
| 28 | `ProjectCategoryTooLong` | Project category too long |
| 29 | `ProjectWebsiteTooLong` | Project website too long |
| 30 | `VerificationNotRevocable` | Project is not in a revocable state (must be Verified) |
| 31 | `TransferNotFound` | No pending ownership transfer found for this project |
| 32 | `NotPendingTransferRecipient` | Caller is not the designated recipient of the pending transfer |
| 33 | `VerificationExpired` | Verification has expired and is no longer active |
| 34 | `ProjectAlreadyExists` | Project with this name or slug already exists |
| 35 | `InvalidCid` | Invalid CID format |
| 36 | `InvalidInput` | Invalid input provided |
| 37 | `InvalidProjectSlug` | Invalid project slug |
| 38 | `InvalidStatus` | Invalid status for the requested operation |
| 39 | `AlreadyArchived` | Project is already archived |
| 40 | `ProjectNotArchived` | Project is not archived |
| 41 | `ProjectTooYoung` | Project is too young for this operation |
| 42 | `VerifiedFieldFrozen` | Verified field is frozen and cannot be modified |
| 43 | `ReservedName` | Project name is reserved |
| 44 | `DuplicateProjectName` | Duplicate project name |
| 45 | `CannotLinkToSelf` | Cannot link a project to itself |
| 46 | `AlreadyLinked` | Projects are already linked |
| 47 | `AlreadyFollowing` | Already following this project |
| 48 | `NotFollowing` | Not following this project |
| 49 | `AlreadyReported` | Review has already been reported |
| 50 | `ReviewAlreadyHidden` | Review is already hidden |
| 51 | `ReviewNotHidden` | Review is not hidden |
| 52 | `CollectionNotFound` | Collection not found |
| 53 | `CollectionExists` | Collection already exists |
| 54 | `AlreadyInCollection` | Project is already in the collection |
| 55 | `ReviewsDisabled` | Reviews are disabled for this project |
| 56 | `OwnerCannotReview` | Project owner cannot review their own project |
| 57 | `InvalidNameFormat` | Invalid name format |
| 58 | `ReviewerNotEligible` | Reviewer is not eligible |
| 59 | `ReviewFeeRequired` | Review fee is required |
| 60 | `CollectionFull` | Collection is full |
| 61 | `ContractPaused` | Contract is paused |
| 62 | `AlreadyBookmarked` | Project is already bookmarked |
| 63 | `AlreadyEndorsed` | Project is already endorsed |
| 64 | `NotBookmarked` | Project is not bookmarked |
| 65 | `NotEndorsed` | Project is not endorsed |
| 66 | `TimelockNotExpired` | Timelock action has not expired yet |
| 67 | `PayloadHashMismatch` | Stored proposal payload does not match its recorded hash |
| 68 | `InvalidTags` | Tag list is invalid (empty, over-length, too many, bad charset, or duplicates) |
| 69 | `ProposalExpired` | Admin proposal has passed its expiry time and can no longer be executed |
| 70 | `NoRefundAvailable` | No refund is recorded for the given project. |
| 71 | `RefundAlreadyClaimed` | The recorded refund has already been paid out. |
| 72 | `ArithmeticOverflow` | A checked arithmetic operation overflowed. |
| 73 | `NotInCollection` | Project is not in the collection |
| 74 | `ThresholdDowngradeRequiresSupermajority` | A SetThreshold proposal that would lower the threshold must be approved by strictly more admins than |
| 75 | `CircularDependency` | Adding this dependency would create a circular reference in the transitive project-dependency graph  |
| 76 | `DependencyDepthExceeded` | Adding this dependency would make the transitive project-dependency chain deeper than `MAX_DEPENDENC |
| 77 | `MultiSigRequired` | Multi-signature admin approval is required for this operation. |
| 78 | `FeePaymentExpired` | The stored fee payment has expired and is no longer valid. |
| 79 | `LinkedProjectNotFound` | The linked project referenced by this operation no longer exists. |
| 80 | `AlreadyMaintainerAdded` | The maintainer is already on the project maintainer list. |
| 81 | `DisputeNotPending` | The dispute is not in a pending state and cannot be resolved. |
| 82 | `TooManyEvidenceLinks` | Too many evidence links supplied (max MAX_EVIDENCE_LINKS_PER_REVIEW). |
| 83 | `InvalidEvidenceLink` | Evidence link URL is empty or has an invalid scheme. |
| 84 | `EvidenceLinkTooLong` | Evidence link URL exceeds MAX_EVIDENCE_LINK_URL_LEN bytes. |
| 85 | `ReviewArchived` | The requested review has been archived and is no longer in primary storage. Use `get_archived_review |
| 86 | `ReviewNotArchived` | The requested review has not been archived; the operation requires an archived review (e.g., `set_ar |
| 87 | `AppealLimitExceeded` | Maximum number of appeals for the current rejection has been reached. The project must follow the ma |
| 88 | `AppealNotFound` | The requested verification appeal does not exist or is no longer valid. |
| 89 | `AppealAlreadyReviewed` | The verification appeal has already been reviewed and cannot be changed. |
| 90 | `AdminLacksExpertise` | Admin lacks the required expertise for this verification assignment. |
| 91 | `AssignmentNotFound` | Verification assignment record not found. |
| 92 | `InvalidAssignmentStatus` | Invalid assignment status for the requested transition. |
| 93 | `SlaNotBreached` | SLA has not been breached yet; cannot escalate without cause. |
| 94 | `NotAssignedAdmin` | Caller is not the assigned admin for this verification. |
| 95 | `NoActiveAssignment` | No active assignment exists for this verification request. |
| 96 | `ParameterOutOfRange` | A governance parameter value was set outside its configured `[min, max]` range (issue #740). Use `ge |
| 97 | `InvalidParamRange` | A governance parameter range was supplied with `min > max`, or the range would leave the parameter's |
| 98 | `RecoveryCaseNotFound` | The referenced project-ownership recovery case does not exist (issue #747). |
| 99 | `RecoveryNotActive` | The ownership-recovery case is not in a state that allows this operation — e.g. voting before the no |
| 100 | `BulkImportValidationFailed` | A bulk import was rejected because at least one entry failed validation. Nothing was written (issue  |
| 101 | `BulkImportTooLarge` | A bulk import exceeded `MAX_BULK_IMPORT_PROJECTS` entries (issue #742). Split the payload across sev |
| 102 | `FolderNotFound` | Bookmark folder not found. |
| 103 | `FolderAlreadyExists` | A folder with this name already exists for the user. |
| 104 | `MaxFoldersExceeded` | The maximum number of bookmark folders per user has been reached. |
| 105 | `FolderDepthExceeded` | Nested folder depth limit exceeded. |
| 106 | `SmartFolderNotFound` | Smart folder not found. |
| 107 | `MaxSmartFoldersExceeded` | The maximum number of smart folders per user has been reached. |
| 108 | `RecommendationNotFound` | Recommendation referenced by the operation does not exist. |
| 109 | `RecommendationFeedbackAlreadyGiven` | User has already submitted thumbs-up / thumbs-down feedback for this recommendation. Feedback cannot |
| 110 | `RecommendationLabelTooLong` | Recommendation label string exceeds the configured length limit. |
| 111 | `RecommendationNoImpression` | Engagement recording rejected because impressions are required before clicks. (Prevents CTR inflatio |
| 112 | `RecommendationInvalidContext` | The combination of audience + reference project specified for the recommendation is invalid (e.g. Si |
| 113 | `RecommendationAudienceMismatch` | Attempted to record feedback from a user who does not match the recommendation's restricted audience |
| 114 | `CommunityColNotFound` | Community collection referenced by the operation does not exist. |
| 115 | `CommunityColNameExists` | Community-collection name already in use by another community collection. |
| 116 | `CommunityColInvalidMetadata` | Community-collection name or description violates configured length limits. |
| 117 | `CommunityColFull` | Community-collection project-membership limit reached; cannot add more. |
| 118 | `CommunityColNotCurator` | Operation restricted to the creator or an active curator but caller is neither. |
| 119 | `CommunityColVoteAlreadyCast` | Voter has already cast a vote for this project in this collection. Voting is append-only per voter p |
| 120 | `CommunityColAlreadyIncluded` | Project is already explicitly included in the community collection. |
| 121 | `CommunityColNotIncluded` | Project is not in the community collection (for removal operations). |
| 122 | `CommunityColCuratorsEmpty` | A curator set can never be empty — removing the last curator (or the creator) would strand the colle |
| 123 | `CommunityColCreatorIsImmutable` | Creator cannot be removed from the curator set — creator status is permanent. |
| 124 | `CommunityColRevenueShareInvalid` | Revenue share cannot exceed 10_000 bps; creator share + per-curator allocation sum must be ≤ 10_000  |
| 125 | `CommunityColThresholdInvalid` | Vote thresholds must both be zero (disabled) or both be > zero (to keep the approval/disapproval sym |
| 126 | `CommunityColIsTemplate` | Template collections cannot accept votes, revenue attribution, or member edits — clone them first to |
| 127 | `CommunityColTemplateUnknown` | The referenced pre-defined template id is not in the `CommunityCollectionTemplateId` enum. |
| 128 | `CommunityColFeaturedCapExceeded` | Admin flagged this community collection as featured but a global cap on featured community collectio |
| 129 | `SocialAnalyticsProjectNotFound` | Social analytics: target project passed to the analytics endpoint does not exist in the registry. |
| 130 | `SocialAnalyticsInvalidWindow` | Social analytics: caller passed `window_start_day > window_end_day`. |
| 131 | `SocialAnalyticsCheckpointCapExceeded` | Social analytics: daily-checkpoint storage has a cap per project so that the per-project index canno |
| 132 | `SocialAnalyticsNoPeers` | Social analytics: peer-comparison set requires at least 1 other project in the same category to run  |
| 133 | `SocialAnalyticsNoCheckpoints` | Social analytics: the export/report endpoint requires at least one checkpoint to produce a "growth o |
| 134 | `InvalidLanguageCode` | Invalid ISO 639-1 language code format (must be exactly 2 lowercase letters). |
| 135 | `VetoLimitExceeded` | Admin has exceeded the monthly veto (rejection) limit (#730). |
| 136 | `CommentLocked` | Comment on a proposal is not allowed because voting has already started. |
| 137 | `EmergencyRecoveryNotFound` | Emergency recovery request not found. |
| 138 | `EmergencyRecoveryNotPending` | Emergency recovery request is not in the pending state. |
| 139 | `ThresholdRequiresUnanimousApproval` | Threshold increase requires all admins to approve. |
| 140 | `RateLimitExceeded` | Caller exceeded the configured per-address rate limit for this operation (issue #945). Wait for the  |

## Errors

### 1 — `AlreadyInitialized`

- **Meaning / root cause:** Contract has already been initialized
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 1.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyInitialized)));
```

### 2 — `Unauthorized`

- **Meaning / root cause:** Unauthorized access - caller is not permitted
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 2.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::Unauthorized)));
```

### 3 — `ProjectNotFound`

- **Meaning / root cause:** Project not found
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 3.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProjectNotFound)));
```

### 4 — `InvalidRating`

- **Meaning / root cause:** Invalid rating - must be between 1 and 5
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 4.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidRating)));
```

### 5 — `ReviewNotFound`

- **Meaning / root cause:** Review not found
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 5.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReviewNotFound)));
```

### 6 — `DuplicateReview`

- **Meaning / root cause:** Duplicate review submission for same project and reviewer
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 6.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::DuplicateReview)));
```

### 7 — `NotReviewOwner`

- **Meaning / root cause:** Caller is not the owner of the targeted review
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 7.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NotReviewOwner)));
```

### 8 — `VerificationNotFound`

- **Meaning / root cause:** Verification record not found
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 8.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::VerificationNotFound)));
```

### 9 — `InvalidStatusTransition`

- **Meaning / root cause:** Invalid verification status transition
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 9.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidStatusTransition)));
```

### 10 — `AdminOnly`

- **Meaning / root cause:** Only admin can perform this action
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 10.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AdminOnly)));
```

### 11 — `FeeConfigNotSet`

- **Meaning / root cause:** Fee configuration not set
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 11.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::FeeConfigNotSet)));
```

### 12 — `TreasuryNotSet`

- **Meaning / root cause:** Treasury address not set
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 12.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::TreasuryNotSet)));
```

### 13 — `InsufficientFee`

- **Meaning / root cause:** Insufficient fee paid
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 13.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InsufficientFee)));
```

### 14 — `InvalidProjectData`

- **Meaning / root cause:** Invalid project data - missing required fields
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 14.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectData)));
```

### 15 — `ProjectNameTooLong`

- **Meaning / root cause:** Project name too long
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 15.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProjectNameTooLong)));
```

### 16 — `InvalidProjectNameFormat`

- **Meaning / root cause:** Invalid project name format
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 16.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectNameFormat)));
```

### 17 — `CannotRemoveLastAdmin`

- **Meaning / root cause:** Cannot remove last admin
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 17.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CannotRemoveLastAdmin)));
```

### 18 — `AdminNotFound`

- **Meaning / root cause:** Admin not found
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 18.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AdminNotFound)));
```

### 19 — `InvalidProjectName`

- **Meaning / root cause:** Invalid project name - empty or whitespace only
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 19.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectName)));
```

### 20 — `InvalidProjectDescription`

- **Meaning / root cause:** Invalid project description - empty or whitespace only
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 20.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectDescription)));
```

### 21 — `InvalidProjectCategory`

- **Meaning / root cause:** Invalid project category - empty or whitespace only
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 21.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectCategory)));
```

### 22 — `ProjectDescriptionTooLong`

- **Meaning / root cause:** Project description too long
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 22.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProjectDescriptionTooLong)));
```

### 23 — `InvalidProjectDescriptionFormat`

- **Meaning / root cause:** Project description contains invalid characters
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 23.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectDescriptionFormat)));
```

### 24 — `MaxProjectsExceeded`

- **Meaning / root cause:** Maximum number of projects exceeded
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 24.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::MaxProjectsExceeded)));
```

### 25 — `InvalidProjectWebsite`

- **Meaning / root cause:** Invalid project website
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 25.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectWebsite)));
```

### 26 — `InvalidProjectLogoCid`

- **Meaning / root cause:** Invalid project logo CID
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 26.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectLogoCid)));
```

### 27 — `InvalidProjectMetadataCid`

- **Meaning / root cause:** Invalid project metadata CID
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 27.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectMetadataCid)));
```

### 28 — `ProjectCategoryTooLong`

- **Meaning / root cause:** Project category too long
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 28.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProjectCategoryTooLong)));
```

### 29 — `ProjectWebsiteTooLong`

- **Meaning / root cause:** Project website too long
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 29.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProjectWebsiteTooLong)));
```

### 30 — `VerificationNotRevocable`

- **Meaning / root cause:** Project is not in a revocable state (must be Verified)
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 30.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::VerificationNotRevocable)));
```

### 31 — `TransferNotFound`

- **Meaning / root cause:** No pending ownership transfer found for this project
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 31.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::TransferNotFound)));
```

### 32 — `NotPendingTransferRecipient`

- **Meaning / root cause:** Caller is not the designated recipient of the pending transfer
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 32.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NotPendingTransferRecipient)));
```

### 33 — `VerificationExpired`

- **Meaning / root cause:** Verification has expired and is no longer active
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 33.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::VerificationExpired)));
```

### 34 — `ProjectAlreadyExists`

- **Meaning / root cause:** Project with this name or slug already exists
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 34.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProjectAlreadyExists)));
```

### 35 — `InvalidCid`

- **Meaning / root cause:** Invalid CID format
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 35.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidCid)));
```

### 36 — `InvalidInput`

- **Meaning / root cause:** Invalid input provided
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 36.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidInput)));
```

### 37 — `InvalidProjectSlug`

- **Meaning / root cause:** Invalid project slug
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 37.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidProjectSlug)));
```

### 38 — `InvalidStatus`

- **Meaning / root cause:** Invalid status for the requested operation
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 38.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidStatus)));
```

### 39 — `AlreadyArchived`

- **Meaning / root cause:** Project is already archived
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 39.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyArchived)));
```

### 40 — `ProjectNotArchived`

- **Meaning / root cause:** Project is not archived
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 40.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProjectNotArchived)));
```

### 41 — `ProjectTooYoung`

- **Meaning / root cause:** Project is too young for this operation
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 41.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProjectTooYoung)));
```

### 42 — `VerifiedFieldFrozen`

- **Meaning / root cause:** Verified field is frozen and cannot be modified
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 42.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::VerifiedFieldFrozen)));
```

### 43 — `ReservedName`

- **Meaning / root cause:** Project name is reserved
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 43.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReservedName)));
```

### 44 — `DuplicateProjectName`

- **Meaning / root cause:** Duplicate project name
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 44.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::DuplicateProjectName)));
```

### 45 — `CannotLinkToSelf`

- **Meaning / root cause:** Cannot link a project to itself
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 45.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CannotLinkToSelf)));
```

### 46 — `AlreadyLinked`

- **Meaning / root cause:** Projects are already linked
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 46.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyLinked)));
```

### 47 — `AlreadyFollowing`

- **Meaning / root cause:** Already following this project
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 47.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyFollowing)));
```

### 48 — `NotFollowing`

- **Meaning / root cause:** Not following this project
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 48.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NotFollowing)));
```

### 49 — `AlreadyReported`

- **Meaning / root cause:** Review has already been reported
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 49.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyReported)));
```

### 50 — `ReviewAlreadyHidden`

- **Meaning / root cause:** Review is already hidden
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 50.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReviewAlreadyHidden)));
```

### 51 — `ReviewNotHidden`

- **Meaning / root cause:** Review is not hidden
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 51.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReviewNotHidden)));
```

### 52 — `CollectionNotFound`

- **Meaning / root cause:** Collection not found
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 52.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CollectionNotFound)));
```

### 53 — `CollectionExists`

- **Meaning / root cause:** Collection already exists
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 53.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CollectionExists)));
```

### 54 — `AlreadyInCollection`

- **Meaning / root cause:** Project is already in the collection
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 54.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyInCollection)));
```

### 55 — `ReviewsDisabled`

- **Meaning / root cause:** Reviews are disabled for this project
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 55.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReviewsDisabled)));
```

### 56 — `OwnerCannotReview`

- **Meaning / root cause:** Project owner cannot review their own project
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 56.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::OwnerCannotReview)));
```

### 57 — `InvalidNameFormat`

- **Meaning / root cause:** Invalid name format
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 57.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidNameFormat)));
```

### 58 — `ReviewerNotEligible`

- **Meaning / root cause:** Reviewer is not eligible
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 58.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReviewerNotEligible)));
```

### 59 — `ReviewFeeRequired`

- **Meaning / root cause:** Review fee is required
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 59.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReviewFeeRequired)));
```

### 60 — `CollectionFull`

- **Meaning / root cause:** Collection is full
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 60.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CollectionFull)));
```

### 61 — `ContractPaused`

- **Meaning / root cause:** Contract is paused
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 61.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ContractPaused)));
```

### 62 — `AlreadyBookmarked`

- **Meaning / root cause:** Project is already bookmarked
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 62.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyBookmarked)));
```

### 63 — `AlreadyEndorsed`

- **Meaning / root cause:** Project is already endorsed
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 63.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyEndorsed)));
```

### 64 — `NotBookmarked`

- **Meaning / root cause:** Project is not bookmarked
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 64.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NotBookmarked)));
```

### 65 — `NotEndorsed`

- **Meaning / root cause:** Project is not endorsed
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 65.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NotEndorsed)));
```

### 66 — `TimelockNotExpired`

- **Meaning / root cause:** Timelock action has not expired yet
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 66.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::TimelockNotExpired)));
```

### 67 — `PayloadHashMismatch`

- **Meaning / root cause:** Stored proposal payload does not match its recorded hash
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 67.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::PayloadHashMismatch)));
```

### 68 — `InvalidTags`

- **Meaning / root cause:** Tag list is invalid (empty, over-length, too many, bad charset, or duplicates)
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 68.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidTags)));
```

### 69 — `ProposalExpired`

- **Meaning / root cause:** Admin proposal has passed its expiry time and can no longer be executed
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 69.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ProposalExpired)));
```

### 70 — `NoRefundAvailable`

- **Meaning / root cause:** No refund is recorded for the given project.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 70.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NoRefundAvailable)));
```

### 71 — `RefundAlreadyClaimed`

- **Meaning / root cause:** The recorded refund has already been paid out.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 71.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RefundAlreadyClaimed)));
```

### 72 — `ArithmeticOverflow`

- **Meaning / root cause:** A checked arithmetic operation overflowed.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 72.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ArithmeticOverflow)));
```

### 73 — `NotInCollection`

- **Meaning / root cause:** Project is not in the collection
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 73.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NotInCollection)));
```

### 74 — `ThresholdDowngradeRequiresSupermajority`

- **Meaning / root cause:** A SetThreshold proposal that would lower the threshold must be approved by strictly more admins than the proposed new threshold (supermajority rule). This prevents the multi-sig quorum from being silently dismantled by exactly the number of colluding admins it is meant to require.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 74.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ThresholdDowngradeRequiresSupermajority)));
```

### 75 — `CircularDependency`

- **Meaning / root cause:** Adding this dependency would create a circular reference in the transitive project-dependency graph (project A depends on B which depends back on A, directly or indirectly).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 75.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CircularDependency)));
```

### 76 — `DependencyDepthExceeded`

- **Meaning / root cause:** Adding this dependency would make the transitive project-dependency chain deeper than `MAX_DEPENDENCY_DEPTH` levels.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 76.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::DependencyDepthExceeded)));
```

### 77 — `MultiSigRequired`

- **Meaning / root cause:** Multi-signature admin approval is required for this operation.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 77.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::MultiSigRequired)));
```

### 78 — `FeePaymentExpired`

- **Meaning / root cause:** The stored fee payment has expired and is no longer valid.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 78.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::FeePaymentExpired)));
```

### 79 — `LinkedProjectNotFound`

- **Meaning / root cause:** The linked project referenced by this operation no longer exists.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 79.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::LinkedProjectNotFound)));
```

### 80 — `AlreadyMaintainerAdded`

- **Meaning / root cause:** The maintainer is already on the project maintainer list.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 80.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AlreadyMaintainerAdded)));
```

### 81 — `DisputeNotPending`

- **Meaning / root cause:** The dispute is not in a pending state and cannot be resolved.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 81.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::DisputeNotPending)));
```

### 82 — `TooManyEvidenceLinks`

- **Meaning / root cause:** Too many evidence links supplied (max MAX_EVIDENCE_LINKS_PER_REVIEW).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 82.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::TooManyEvidenceLinks)));
```

### 83 — `InvalidEvidenceLink`

- **Meaning / root cause:** Evidence link URL is empty or has an invalid scheme.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 83.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidEvidenceLink)));
```

### 84 — `EvidenceLinkTooLong`

- **Meaning / root cause:** Evidence link URL exceeds MAX_EVIDENCE_LINK_URL_LEN bytes.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 84.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::EvidenceLinkTooLong)));
```

### 85 — `ReviewArchived`

- **Meaning / root cause:** The requested review has been archived and is no longer in primary storage. Use `get_archived_review` to retrieve the compact archived record.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 85.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReviewArchived)));
```

### 86 — `ReviewNotArchived`

- **Meaning / root cause:** The requested review has not been archived; the operation requires an archived review (e.g., `set_archived_review_arweave_tx`).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 86.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ReviewNotArchived)));
```

### 87 — `AppealLimitExceeded`

- **Meaning / root cause:** Maximum number of appeals for the current rejection has been reached. The project must follow the manual review process after the cap is hit.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 87.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AppealLimitExceeded)));
```

### 88 — `AppealNotFound`

- **Meaning / root cause:** The requested verification appeal does not exist or is no longer valid.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 88.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AppealNotFound)));
```

### 89 — `AppealAlreadyReviewed`

- **Meaning / root cause:** The verification appeal has already been reviewed and cannot be changed.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 89.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AppealAlreadyReviewed)));
```

### 90 — `AdminLacksExpertise`

- **Meaning / root cause:** Admin lacks the required expertise for this verification assignment.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 90.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AdminLacksExpertise)));
```

### 91 — `AssignmentNotFound`

- **Meaning / root cause:** Verification assignment record not found.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 91.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::AssignmentNotFound)));
```

### 92 — `InvalidAssignmentStatus`

- **Meaning / root cause:** Invalid assignment status for the requested transition.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 92.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidAssignmentStatus)));
```

### 93 — `SlaNotBreached`

- **Meaning / root cause:** SLA has not been breached yet; cannot escalate without cause.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 93.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::SlaNotBreached)));
```

### 94 — `NotAssignedAdmin`

- **Meaning / root cause:** Caller is not the assigned admin for this verification.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 94.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NotAssignedAdmin)));
```

### 95 — `NoActiveAssignment`

- **Meaning / root cause:** No active assignment exists for this verification request.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 95.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::NoActiveAssignment)));
```

### 96 — `ParameterOutOfRange`

- **Meaning / root cause:** A governance parameter value was set outside its configured `[min, max]` range (issue #740). Use `get_param_range` to read the bounds currently in force.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 96.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ParameterOutOfRange)));
```

### 97 — `InvalidParamRange`

- **Meaning / root cause:** A governance parameter range was supplied with `min > max`, or the range would leave the parameter's current value invalid (issue #740).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 97.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidParamRange)));
```

### 98 — `RecoveryCaseNotFound`

- **Meaning / root cause:** The referenced project-ownership recovery case does not exist (issue #747).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 98.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RecoveryCaseNotFound)));
```

### 99 — `RecoveryNotActive`

- **Meaning / root cause:** The ownership-recovery case is not in a state that allows this operation — e.g. voting before the nomination reached `RECOVERY_REQUIRED_ENDORSEMENTS`, or a second vote from the same address (issue #747).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 99.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RecoveryNotActive)));
```

### 100 — `BulkImportValidationFailed`

- **Meaning / root cause:** A bulk import was rejected because at least one entry failed validation. Nothing was written (issue #742); the `validate_bulk_import` dry run reports the individual failures.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 100.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::BulkImportValidationFailed)));
```

### 101 — `BulkImportTooLarge`

- **Meaning / root cause:** A bulk import exceeded `MAX_BULK_IMPORT_PROJECTS` entries (issue #742). Split the payload across several calls.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 101.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::BulkImportTooLarge)));
```

### 102 — `FolderNotFound`

- **Meaning / root cause:** Bookmark folder not found.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 102.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::FolderNotFound)));
```

### 103 — `FolderAlreadyExists`

- **Meaning / root cause:** A folder with this name already exists for the user.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 103.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::FolderAlreadyExists)));
```

### 104 — `MaxFoldersExceeded`

- **Meaning / root cause:** The maximum number of bookmark folders per user has been reached.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 104.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::MaxFoldersExceeded)));
```

### 105 — `FolderDepthExceeded`

- **Meaning / root cause:** Nested folder depth limit exceeded.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 105.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::FolderDepthExceeded)));
```

### 106 — `SmartFolderNotFound`

- **Meaning / root cause:** Smart folder not found.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 106.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::SmartFolderNotFound)));
```

### 107 — `MaxSmartFoldersExceeded`

- **Meaning / root cause:** The maximum number of smart folders per user has been reached.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 107.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::MaxSmartFoldersExceeded)));
```

### 108 — `RecommendationNotFound`

- **Meaning / root cause:** Recommendation referenced by the operation does not exist.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 108.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RecommendationNotFound)));
```

### 109 — `RecommendationFeedbackAlreadyGiven`

- **Meaning / root cause:** User has already submitted thumbs-up / thumbs-down feedback for this recommendation. Feedback cannot be rewritten (see issue #820 — feedback is append-only to preserve an auditable paper-trail for recommendation improvement).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 109.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RecommendationFeedbackAlreadyGiven)));
```

### 110 — `RecommendationLabelTooLong`

- **Meaning / root cause:** Recommendation label string exceeds the configured length limit.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 110.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RecommendationLabelTooLong)));
```

### 111 — `RecommendationNoImpression`

- **Meaning / root cause:** Engagement recording rejected because impressions are required before clicks. (Prevents CTR inflation via click-only spamming.)
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 111.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RecommendationNoImpression)));
```

### 112 — `RecommendationInvalidContext`

- **Meaning / root cause:** The combination of audience + reference project specified for the recommendation is invalid (e.g. Similar recs require a reference project).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 112.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RecommendationInvalidContext)));
```

### 113 — `RecommendationAudienceMismatch`

- **Meaning / root cause:** Attempted to record feedback from a user who does not match the recommendation's restricted audience (when audience.is_some).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 113.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RecommendationAudienceMismatch)));
```

### 114 — `CommunityColNotFound`

- **Meaning / root cause:** Community collection referenced by the operation does not exist.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 114.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColNotFound)));
```

### 115 — `CommunityColNameExists`

- **Meaning / root cause:** Community-collection name already in use by another community collection.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 115.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColNameExists)));
```

### 116 — `CommunityColInvalidMetadata`

- **Meaning / root cause:** Community-collection name or description violates configured length limits.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 116.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColInvalidMetadata)));
```

### 117 — `CommunityColFull`

- **Meaning / root cause:** Community-collection project-membership limit reached; cannot add more.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 117.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColFull)));
```

### 118 — `CommunityColNotCurator`

- **Meaning / root cause:** Operation restricted to the creator or an active curator but caller is neither.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 118.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColNotCurator)));
```

### 119 — `CommunityColVoteAlreadyCast`

- **Meaning / root cause:** Voter has already cast a vote for this project in this collection. Voting is append-only per voter per project per collection.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 119.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColVoteAlreadyCast)));
```

### 120 — `CommunityColAlreadyIncluded`

- **Meaning / root cause:** Project is already explicitly included in the community collection.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 120.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColAlreadyIncluded)));
```

### 121 — `CommunityColNotIncluded`

- **Meaning / root cause:** Project is not in the community collection (for removal operations).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 121.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColNotIncluded)));
```

### 122 — `CommunityColCuratorsEmpty`

- **Meaning / root cause:** A curator set can never be empty — removing the last curator (or the creator) would strand the collection.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 122.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColCuratorsEmpty)));
```

### 123 — `CommunityColCreatorIsImmutable`

- **Meaning / root cause:** Creator cannot be removed from the curator set — creator status is permanent.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 123.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColCreatorIsImmutable)));
```

### 124 — `CommunityColRevenueShareInvalid`

- **Meaning / root cause:** Revenue share cannot exceed 10_000 bps; creator share + per-curator allocation sum must be ≤ 10_000 bps.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 124.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColRevenueShareInvalid)));
```

### 125 — `CommunityColThresholdInvalid`

- **Meaning / root cause:** Vote thresholds must both be zero (disabled) or both be > zero (to keep the approval/disapproval symmetry obvious and avoid accidental open-gate configurations).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 125.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColThresholdInvalid)));
```

### 126 — `CommunityColIsTemplate`

- **Meaning / root cause:** Template collections cannot accept votes, revenue attribution, or member edits — clone them first to produce a working collection.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 126.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColIsTemplate)));
```

### 127 — `CommunityColTemplateUnknown`

- **Meaning / root cause:** The referenced pre-defined template id is not in the `CommunityCollectionTemplateId` enum.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 127.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColTemplateUnknown)));
```

### 128 — `CommunityColFeaturedCapExceeded`

- **Meaning / root cause:** Admin flagged this community collection as featured but a global cap on featured community collections was reached.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 128.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommunityColFeaturedCapExceeded)));
```

### 129 — `SocialAnalyticsProjectNotFound`

- **Meaning / root cause:** Social analytics: target project passed to the analytics endpoint does not exist in the registry.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 129.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::SocialAnalyticsProjectNotFound)));
```

### 130 — `SocialAnalyticsInvalidWindow`

- **Meaning / root cause:** Social analytics: caller passed `window_start_day > window_end_day`.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 130.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::SocialAnalyticsInvalidWindow)));
```

### 131 — `SocialAnalyticsCheckpointCapExceeded`

- **Meaning / root cause:** Social analytics: daily-checkpoint storage has a cap per project so that the per-project index cannot grow indefinitely.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 131.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::SocialAnalyticsCheckpointCapExceeded)));
```

### 132 — `SocialAnalyticsNoPeers`

- **Meaning / root cause:** Social analytics: peer-comparison set requires at least 1 other project in the same category to run a meaningful comparison.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 132.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::SocialAnalyticsNoPeers)));
```

### 133 — `SocialAnalyticsNoCheckpoints`

- **Meaning / root cause:** Social analytics: the export/report endpoint requires at least one checkpoint to produce a "growth over time" report. Callers may run the checkpoint endpoint first.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 133.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::SocialAnalyticsNoCheckpoints)));
```

### 134 — `InvalidLanguageCode`

- **Meaning / root cause:** Invalid ISO 639-1 language code format (must be exactly 2 lowercase letters).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 134.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::InvalidLanguageCode)));
```

### 135 — `VetoLimitExceeded`

- **Meaning / root cause:** Admin has exceeded the monthly veto (rejection) limit (#730).
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 135.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::VetoLimitExceeded)));
```

### 136 — `CommentLocked`

- **Meaning / root cause:** Comment on a proposal is not allowed because voting has already started.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 136.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::CommentLocked)));
```

### 137 — `EmergencyRecoveryNotFound`

- **Meaning / root cause:** Emergency recovery request not found.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 137.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::EmergencyRecoveryNotFound)));
```

### 138 — `EmergencyRecoveryNotPending`

- **Meaning / root cause:** Emergency recovery request is not in the pending state.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 138.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::EmergencyRecoveryNotPending)));
```

### 139 — `ThresholdRequiresUnanimousApproval`

- **Meaning / root cause:** Threshold increase requires all admins to approve.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 139.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::ThresholdRequiresUnanimousApproval)));
```

### 140 — `RateLimitExceeded`

- **Meaning / root cause:** Caller exceeded the configured per-address rate limit for this operation (issue #945). Wait for the window to reset or ask an admin for a trusted-address override.
- **Debugging steps:** (1) Reproduce with the exact inputs and note the returned code. (2) Read the on-chain state for the referenced record (project/review/collection id or slug). (3) Check caller auth and admin/owner permissions. (4) Search this catalog and `docs/error-reference.html` for code 140.
- **Resolution:** Fix the offending input or permission, then retry the operation.
- **Triggering example:**
```rust
// Any call violating the documented precondition returns this error, e.g.:
let res = client.try_operation(&args);
assert_eq!(res, Err(Ok(ContractError::RateLimitExceeded)));
```
