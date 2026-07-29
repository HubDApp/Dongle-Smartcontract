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
    AlreadyInitialized = 1,
    NotInitialized = 2,
    ProjectNotFound = 3,
    SlugAlreadyExists = 4,
    MaxProjectsExceeded = 5,
    ReviewNotFound = 6,
    InvalidCid = 7,
    InvalidInput = 8,
    InvalidProjectName = 9,
    InvalidProjectData = 10,
    InvalidProjectSlug = 11,
    InvalidStatus = 12,
    ProjectAlreadyExists = 13,
    AlreadyArchived = 14,
    ProjectNotArchived = 15,
    ProjectTooYoung = 16,
    VerificationNotFound = 17,
    VerifiedFieldFrozen = 18,
    Unauthorized = 19,
    AdminOnly = 20,
    AdminNotFound = 21,
    TransferNotFound = 22,
    ReservedName = 23,
    FeeAlreadyPaid = 24,
    InsufficientFee = 25,
    DuplicateProjectName = 26,
    IndexOutOfBounds = 27,
    CannotLinkToSelf = 28,
    AlreadyLinked = 29,
    AlreadyFollowing = 30,
    NotFollowing = 31,
    DuplicateReview = 32,
    InvalidRating = 33,
    AlreadyReported = 34,
    ReviewAlreadyHidden = 35,
    ReviewNotHidden = 36,
    CollectionNotFound = 37,
    CollectionExists = 38,
    AlreadyInCollection = 39,
    CannotRemoveLastAdmin = 40,
    ReviewsDisabled = 41,
    NotReviewOwner = 42,
    TreasuryNotSet = 43,
    FeeConfigNotSet = 44,
    TooManyTags = 45,
    OwnerCannotReview = 46,
    InvalidNameFormat = 47,
}

pub type Error = ContractError;
