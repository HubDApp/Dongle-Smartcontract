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
    /// Max projects per user exceeded
    MaxProjectsExceeded = 5,
    /// Review not found
    ReviewNotFound = 6,
    /// Invalid CID format
    InvalidCid = 7,
    /// Invalid input data
    InvalidInput = 8,
    /// Invalid project name - empty, whitespace only, or invalid format
    InvalidProjectName = 9,
    /// Invalid project data - missing required fields or invalid format
    InvalidProjectData = 10,
    /// Invalid project slug
    InvalidProjectSlug = 11,
    /// Invalid status or status transition
    InvalidStatus = 12,
    /// Project already exists
    ProjectAlreadyExists = 13,
    /// Project is already archived
    AlreadyArchived = 14,
    /// Project is not archived
    ProjectNotArchived = 15,
    /// Project is too young to be verified
    ProjectTooYoung = 16,
    /// Verification record not found
    VerificationNotFound = 17,
    /// Field is frozen for verified projects
    VerifiedFieldFrozen = 18,
    /// Only admin can perform this action
    AdminOnly = 19,
    /// Admin not found
    AdminNotFound = 20,
    /// No pending ownership transfer found for this project
    TransferNotFound = 21,
    /// Reserved project name
    ReservedName = 22,
    /// Invalid verification status transition
    InvalidStatusTransition = 23,
    /// Insufficient fee paid
    InsufficientFee = 24,
    /// Treasury address not set
    TreasuryNotSet = 25,
    /// Fee configuration not set
    FeeConfigNotSet = 26,
    /// Duplicate project name
    DuplicateProjectName = 27,
    /// Cannot link a project to itself
    CannotLinkToSelf = 28,
    /// Projects are already linked
    AlreadyLinked = 29,
    /// Already following this project
    AlreadyFollowing = 30,
    /// Not following this project
    NotFollowing = 31,
    /// Duplicate review submission for same project and reviewer
    DuplicateReview = 32,
    /// Already reported this item
    AlreadyReported = 33,
    /// Review is already hidden
    ReviewAlreadyHidden = 34,
    /// Review is not hidden
    ReviewNotHidden = 35,
    /// Collection not found
    CollectionNotFound = 36,
    /// Collection already exists
    CollectionExists = 37,
    /// Project already in collection
    AlreadyInCollection = 38,
    /// Cannot remove the last admin
    CannotRemoveLastAdmin = 39,
    /// Reviews are disabled for this project
    ReviewsDisabled = 40,
    /// Caller is not the owner of the targeted review
    NotReviewOwner = 41,
    /// Caller is not the designated recipient of the pending transfer
    NotPendingTransferRecipient = 42,
    /// Verification has expired and is no longer active
    VerificationExpired = 43,
    /// Project is not in a revocable state (must be Verified)
    VerificationNotRevocable = 44,
    /// Owner cannot review their own project
    OwnerCannotReview = 45,
    /// Invalid name format
    InvalidNameFormat = 46,
    /// Reviewer is not eligible
    ReviewerNotEligible = 47,
    /// Review fee is required
    ReviewFeeRequired = 48,
    /// Collection is full
    CollectionFull = 49,
    /// Contract is paused
    ContractPaused = 50,
    /// Already bookmarked
    AlreadyBookmarked = 51,
    /// Not bookmarked
    NotBookmarked = 52,
    /// Already endorsed
    AlreadyEndorsed = 53,
    /// Not endorsed
    NotEndorsed = 54,
    /// Timelock has not expired yet
    TimelockNotExpired = 55,
}

pub type Error = ContractError;
