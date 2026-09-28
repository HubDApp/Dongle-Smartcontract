use soroban_sdk::contracterror;

#[contracterror(export = false)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Contract has already been initialized
    AlreadyInitialized = 1,
    /// Unauthorized access - caller is not permitted
    Unauthorized = 2,
    /// Project not found
    ProjectNotFound = 3,
    /// Invalid rating - must be between 1 and 5
    InvalidRating = 4,
    /// Review not found
    ReviewNotFound = 5,
    /// Duplicate review submission for same project and reviewer
    DuplicateReview = 6,
    /// Caller is not the owner of the targeted review
    NotReviewOwner = 7,
    /// Verification record not found
    VerificationNotFound = 8,
    /// Invalid verification status transition
    InvalidStatusTransition = 9,
    /// Only admin can perform this action
    AdminOnly = 10,
    /// Fee configuration not set
    FeeConfigNotSet = 11,
    /// Treasury address not set
    TreasuryNotSet = 12,
    /// Insufficient fee paid
    InsufficientFee = 13,
    /// Invalid project data - missing required fields
    InvalidProjectData = 14,
    /// Project name too long
    ProjectNameTooLong = 15,
    /// Invalid project name format
    InvalidProjectNameFormat = 16,
    /// Cannot remove last admin
    CannotRemoveLastAdmin = 17,
    /// Admin not found
    AdminNotFound = 18,
    /// Invalid project name - empty or whitespace only
    InvalidProjectName = 19,
    /// Invalid project description - empty or whitespace only
    InvalidProjectDescription = 20,
    /// Invalid project category - empty or whitespace only
    InvalidProjectCategory = 21,
    /// Project description too long
    ProjectDescriptionTooLong = 22,
    /// Project description contains invalid characters
    InvalidProjectDescriptionFormat = 23,
    /// Maximum number of projects exceeded
    MaxProjectsExceeded = 24,
    /// Invalid project website
    InvalidProjectWebsite = 25,
    /// Invalid project logo CID
    InvalidProjectLogoCid = 26,
    /// Invalid project metadata CID
    InvalidProjectMetadataCid = 27,
    /// Project category too long
    ProjectCategoryTooLong = 28,
    /// Project website too long
    ProjectWebsiteTooLong = 29,
    /// Project is not in a revocable state (must be Verified)
    VerificationNotRevocable = 30,
    /// No pending ownership transfer found for this project
    TransferNotFound = 31,
    /// Caller is not the designated recipient of the pending transfer
    NotPendingTransferRecipient = 32,
    /// Verification has expired and is no longer active
    VerificationExpired = 33,
    /// Project with this name or slug already exists
    ProjectAlreadyExists = 34,
    /// Invalid CID format
    InvalidCid = 35,
    /// Invalid input provided
    InvalidInput = 36,
    /// Invalid project slug
    InvalidProjectSlug = 37,
    /// Invalid status for the requested operation
    InvalidStatus = 38,
    /// Project is already archived
    AlreadyArchived = 39,
    /// Project is not archived
    ProjectNotArchived = 40,
    /// Project is too young for this operation
    ProjectTooYoung = 41,
    /// Verified field is frozen and cannot be modified
    VerifiedFieldFrozen = 42,
    /// Project name is reserved
    ReservedName = 43,
    /// Duplicate project name
    DuplicateProjectName = 44,
    /// Cannot link a project to itself
    CannotLinkToSelf = 45,
    /// Projects are already linked
    AlreadyLinked = 46,
    /// Already following this project
    AlreadyFollowing = 47,
    /// Not following this project
    NotFollowing = 48,
    /// Review has already been reported
    AlreadyReported = 49,
    /// Review is already hidden
    ReviewAlreadyHidden = 50,
    /// Review is not hidden
    ReviewNotHidden = 51,
    /// Collection not found
    CollectionNotFound = 52,
    /// Collection already exists
    CollectionExists = 53,
    /// Project is already in the collection
    AlreadyInCollection = 54,
    /// Reviews are disabled for this project
    ReviewsDisabled = 55,
    /// Project owner cannot review their own project
    OwnerCannotReview = 56,
    /// Invalid name format
    InvalidNameFormat = 57,
    /// Reviewer is not eligible
    ReviewerNotEligible = 58,
    /// Review fee is required
    ReviewFeeRequired = 59,
    /// Collection is full
    CollectionFull = 60,
    /// Contract is paused
    ContractPaused = 61,
    /// Project is already bookmarked
    AlreadyBookmarked = 62,
    /// Project is already endorsed
    AlreadyEndorsed = 63,
    /// Project is not bookmarked
    NotBookmarked = 64,
    /// Project is not endorsed
    NotEndorsed = 65,
    /// Timelock action has not expired yet
    TimelockNotExpired = 66,
    /// Stored proposal payload does not match its recorded hash
    PayloadHashMismatch = 67,
    /// Tag list is invalid (empty, over-length, too many, bad charset, or duplicates)
    InvalidTags = 68,
    /// Admin proposal has passed its expiry time and can no longer be executed
    ProposalExpired = 69,
    /// No refund is recorded for the given project.
    NoRefundAvailable = 70,
    /// The recorded refund has already been paid out.
    RefundAlreadyClaimed = 71,
    /// A checked arithmetic operation overflowed.
    ArithmeticOverflow = 72,
    /// Project is not in the collection
    NotInCollection = 73,
    /// A SetThreshold proposal that would lower the threshold must be approved
    /// by strictly more admins than the proposed new threshold (supermajority
    /// rule). This prevents the multi-sig quorum from being silently dismantled
    /// by exactly the number of colluding admins it is meant to require.
    ThresholdDowngradeRequiresSupermajority = 74,
    /// Adding this dependency would create a circular reference in the
    /// transitive project-dependency graph (project A depends on B which
    /// depends back on A, directly or indirectly).
    CircularDependency = 75,
    /// Adding this dependency would make the transitive project-dependency
    /// chain deeper than `MAX_DEPENDENCY_DEPTH` levels.
    DependencyDepthExceeded = 76,
    /// Multi-signature admin approval is required for this operation.
    MultiSigRequired = 77,
    /// The stored fee payment has expired and is no longer valid.
    FeePaymentExpired = 78,
    /// The linked project referenced by this operation no longer exists.
    LinkedProjectNotFound = 79,
    /// The maintainer is already on the project maintainer list.
    AlreadyMaintainerAdded = 80,
    /// The dispute is not in a pending state and cannot be resolved.
    DisputeNotPending = 81,
    /// Too many evidence links supplied (max MAX_EVIDENCE_LINKS_PER_REVIEW).
    TooManyEvidenceLinks = 82,
    /// Evidence link URL is empty or has an invalid scheme.
    InvalidEvidenceLink = 83,
    /// Evidence link URL exceeds MAX_EVIDENCE_LINK_URL_LEN bytes.
    EvidenceLinkTooLong = 84,
    /// The requested review has been archived and is no longer in primary storage.
    /// Use `get_archived_review` to retrieve the compact archived record.
    ReviewArchived = 85,
    /// The requested review has not been archived; the operation requires an
    /// archived review (e.g., `set_archived_review_arweave_tx`).
    ReviewNotArchived = 86,
    /// Maximum number of appeals for the current rejection has been reached.
    /// The project must follow the manual review process after the cap is hit.
    AppealLimitExceeded = 87,
    /// The requested verification appeal does not exist or is no longer valid.
    AppealNotFound = 88,
    /// The verification appeal has already been reviewed and cannot be changed.
    AppealAlreadyReviewed = 89,
    /// Admin lacks the required expertise for this verification assignment.
    AdminLacksExpertise = 90,
    /// Verification assignment record not found.
    AssignmentNotFound = 91,
    /// Invalid assignment status for the requested transition.
    InvalidAssignmentStatus = 92,
    /// SLA has not been breached yet; cannot escalate without cause.
    SlaNotBreached = 93,
    /// Caller is not the assigned admin for this verification.
    NotAssignedAdmin = 94,
    /// No active assignment exists for this verification request.
    NoActiveAssignment = 95,
    /// A governance parameter value was set outside its configured
    /// `[min, max]` range (issue #740). Use `get_param_range` to read the
    /// bounds currently in force.
    ParameterOutOfRange = 96,
    /// A governance parameter range was supplied with `min > max`, or the
    /// range would leave the parameter's current value invalid (issue #740).
    InvalidParamRange = 97,
    /// The referenced project-ownership recovery case does not exist (issue #747).
    RecoveryCaseNotFound = 98,
    /// The ownership-recovery case is not in a state that allows this
    /// operation — e.g. voting before the nomination reached
    /// `RECOVERY_REQUIRED_ENDORSEMENTS`, or a second vote from the same
    /// address (issue #747).
    RecoveryNotActive = 99,
    /// A bulk import was rejected because at least one entry failed
    /// validation. Nothing was written (issue #742); the
    /// `validate_bulk_import` dry run reports the individual failures.
    BulkImportValidationFailed = 100,
    /// A bulk import exceeded `MAX_BULK_IMPORT_PROJECTS` entries (issue #742).
    /// Split the payload across several calls.
    BulkImportTooLarge = 101,

    // ── Error codes restored after merge damage ────────────────────────────
    //
    // The recommendation (#820), community-collection (#821) and
    // social-analytics (#822) features were merged with their `errors.rs`
    // hunks lost, so their registries referenced variants that did not exist
    // and the crate did not compile. The variants are restored here and
    // renumbered from 102: the original 82-107 range is now occupied by the
    // appeals (#804) / assignment / range / import / recovery errors that
    // landed afterwards, and an error code is part of the public ABI.
    // ── Bookmark Folder errors (#815) ─────────────────────────────────────
    /// Bookmark folder not found.
    FolderNotFound = 102,
    /// A folder with this name already exists for the user.
    FolderAlreadyExists = 103,
    /// The maximum number of bookmark folders per user has been reached.
    MaxFoldersExceeded = 104,
    /// Nested folder depth limit exceeded.
    FolderDepthExceeded = 105,
    /// Smart folder not found.
    SmartFolderNotFound = 106,
    /// The maximum number of smart folders per user has been reached.
    MaxSmartFoldersExceeded = 107,
    /// Recommendation referenced by the operation does not exist.
    RecommendationNotFound = 108,
    /// User has already submitted thumbs-up / thumbs-down feedback for this recommendation.
    /// Feedback cannot be rewritten (see issue #820 — feedback is append-only to preserve
    /// an auditable paper-trail for recommendation improvement).
    RecommendationFeedbackAlreadyGiven = 109,
    /// Recommendation label string exceeds the configured length limit.
    RecommendationLabelTooLong = 110,
    /// Engagement recording rejected because impressions are required before clicks.
    /// (Prevents CTR inflation via click-only spamming.)
    RecommendationNoImpression = 111,
    /// The combination of audience + reference project specified for the
    /// recommendation is invalid (e.g. Similar recs require a reference project).
    RecommendationInvalidContext = 112,
    /// Attempted to record feedback from a user who does not match the
    /// recommendation's restricted audience (when audience.is_some).
    RecommendationAudienceMismatch = 113,
    /// Community collection referenced by the operation does not exist.
    CommunityColNotFound = 114,
    /// Community-collection name already in use by another community collection.
    CommunityColNameExists = 115,
    /// Community-collection name or description violates configured length limits.
    CommunityColInvalidMetadata = 116,
    /// Community-collection project-membership limit reached; cannot add more.
    CommunityColFull = 117,
    /// Operation restricted to the creator or an active curator but caller is neither.
    CommunityColNotCurator = 118,
    /// Voter has already cast a vote for this project in this collection. Voting
    /// is append-only per voter per project per collection.
    CommunityColVoteAlreadyCast = 119,
    /// Project is already explicitly included in the community collection.
    CommunityColAlreadyIncluded = 120,
    /// Project is not in the community collection (for removal operations).
    CommunityColNotIncluded = 121,
    /// A curator set can never be empty — removing the last curator (or the creator)
    /// would strand the collection.
    CommunityColCuratorsEmpty = 122,
    /// Creator cannot be removed from the curator set — creator status is permanent.
    CommunityColCreatorIsImmutable = 123,
    /// Revenue share cannot exceed 10_000 bps; creator share + per-curator allocation
    /// sum must be ≤ 10_000 bps.
    CommunityColRevenueShareInvalid = 124,
    /// Vote thresholds must both be zero (disabled) or both be > zero (to keep
    /// the approval/disapproval symmetry obvious and avoid accidental open-gate
    /// configurations).
    CommunityColThresholdInvalid = 125,
    /// Template collections cannot accept votes, revenue attribution, or member
    /// edits — clone them first to produce a working collection.
    CommunityColIsTemplate = 126,
    /// The referenced pre-defined template id is not in the
    /// `CommunityCollectionTemplateId` enum.
    CommunityColTemplateUnknown = 127,
    /// Admin flagged this community collection as featured but a global cap on
    /// featured community collections was reached.
    CommunityColFeaturedCapExceeded = 128,
    /// Social analytics: target project passed to the analytics endpoint does
    /// not exist in the registry.
    SocialAnalyticsProjectNotFound = 129,
    /// Social analytics: caller passed `window_start_day > window_end_day`.
    SocialAnalyticsInvalidWindow = 130,
    /// Social analytics: daily-checkpoint storage has a cap per project so that
    /// the per-project index cannot grow indefinitely.
    SocialAnalyticsCheckpointCapExceeded = 131,
    /// Social analytics: peer-comparison set requires at least 1 other project
    /// in the same category to run a meaningful comparison.
    SocialAnalyticsNoPeers = 132,
    /// Social analytics: the export/report endpoint requires at least one
    /// checkpoint to produce a "growth over time" report. Callers may run the
    /// checkpoint endpoint first.
    SocialAnalyticsNoCheckpoints = 133,
    /// Admin has exceeded the monthly veto (rejection) limit (#730).
    VetoLimitExceeded = 96,
    /// Comment on a proposal is not allowed because voting has already started.
    CommentLocked = 97,
    /// Emergency recovery request not found.
    EmergencyRecoveryNotFound = 98,
    /// Emergency recovery request is not in the pending state.
    EmergencyRecoveryNotPending = 99,
    /// Threshold increase requires all admins to approve.
    ThresholdRequiresUnanimousApproval = 100,
}

pub type Error = ContractError;
