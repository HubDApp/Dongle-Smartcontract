use soroban_sdk::contracterror;

/// Error types for the Dongle smart contract
#[contracterror]
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
    /// Project too young for verification
    ProjectTooYoung = 18,
    /// Invalid tag format or length
    InvalidTag = 19,
    /// Too many tags
    TooManyTags = 20,
    /// Invalid social link format
    InvalidSocialLink = 21,
    /// Too many social links
    TooManySocialLinks = 22,
    /// Project already reported by this user
    ProjectAlreadyReported = 23,
    /// Invalid report reason
    InvalidReportReason = 24,
    /// Admin not found
    AdminNotFound = 25,
    /// Invalid project name - empty or whitespace only
    InvalidProjectName = 26,
    /// Invalid project description - empty or whitespace only
    InvalidProjectDescription = 27,
    /// Invalid project category - empty or whitespace only
    InvalidProjectCategory = 28,
    /// Project description too long
    ProjectDescriptionTooLong = 29,
    /// Project description contains invalid characters
    InvalidProjectDescriptionFormat = 30,
    MaxProjectsExceeded = 31,
    /// Invalid project website
    InvalidProjectWebsite = 32,
    /// Invalid project logo CID
    InvalidProjectLogoCid = 33,
    /// Invalid project metadata CID
    InvalidProjectMetadataCid = 34,
    /// Project category too long
    ProjectCategoryTooLong = 35,
    /// Project website too long
    ProjectWebsiteTooLong = 36,
    /// Project is not in a revocable state (must be Verified)
    VerificationNotRevocable = 37,
    /// No pending ownership transfer found for this project
    TransferNotFound = 38,
    /// Caller is not the designated recipient of the pending transfer
    NotPendingTransferRecipient = 39,
    /// Reviews are disabled for this project
    ReviewsDisabled = 40,
    /// Review has already been reported by this reporter
    ReviewAlreadyReported = 41,
    /// Review is already hidden
    ReviewAlreadyHidden = 42,
    /// Review is not hidden
    ReviewNotHidden = 43,
    /// Project is already archived
    ProjectAlreadyArchived = 44,
    /// Project is not archived
    ProjectNotArchived = 45,
    /// Reports have already been cleared or there are none to clear
    ReportsAlreadyCleared = 46,
    /// Project is not claimable
    ProjectNotClaimable = 47,
    /// Claim request not found
    ClaimRequestNotFound = 48,
    /// Claim request already exists for this project and claimant
    ClaimRequestAlreadyExists = 49,
    /// Claim request is not pending
    ClaimRequestNotPending = 50,
}

// Legacy alias to avoid breaking any code that uses `Error` directly
pub type Error = ContractError;
