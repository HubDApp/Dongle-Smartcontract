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
    /// Automatic archival is disabled by the current configuration.
    AutoArchiveDisabled = 90,
    /// The project is not yet inside its 30-day pre-archival notice window.
    AutoArchiveNotDue = 91,
    /// No valid pre-archival notice exists for the project.
    AutoArchiveNoticeNotFound = 92,
    /// The 30-day notice period has not elapsed since notification.
    AutoArchiveNoticeTooEarly = 93,
    /// Configured inactivity threshold is outside allowed bounds.
    AutoArchiveThresholdInvalid = 94,
}

pub type Error = ContractError;
