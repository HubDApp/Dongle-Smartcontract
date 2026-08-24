use soroban_sdk::contracterror;

#[contracterror(export = false)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Project not found
    ProjectNotFound = 1,
    /// Unauthorized access - caller is not permitted
    Unauthorized = 2,
    /// Project already exists
    ProjectAlreadyExists = 3,
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
    /// Contract has already been initialized
    AlreadyInitialized = 34,
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
}

pub type Error = ContractError;
