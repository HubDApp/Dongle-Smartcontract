 use soroban_sdk::contracterror;

/// Error types for the Dongle smart contract
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    AlreadyReviewed = 19
    /// Project not found
    ProjectNotFound = 1,

    /// Unauthorized access - caller is not the owner
    Unauthorized = 2,

    /// Project already exists
    ProjectAlreadyExists = 3,

    /// Invalid rating - must be between 1 and 5
    InvalidRating = 4,

    /// Review not found
    ReviewNotFound = 5,

    /// Review already exists
    ReviewAlreadyExists = 6,

    /// Verification record not found
    VerificationNotFound = 7,

    /// Invalid verification status transition
    InvalidStatusTransition = 8,

    /// Only admin can perform this action
    AdminOnly = 9,

    /// Invalid fee amount
    InvalidFeeAmount = 10,

    /// Insufficient fee paid
    InsufficientFee = 11,

    /// Invalid project data - missing required fields
    InvalidProjectData = 12,

    /// Project name too long
    ProjectNameTooLong = 13,

    /// Project description too long
    ProjectDescriptionTooLong = 14,

    /// Invalid project category
    InvalidProjectCategory = 15,

    /// Verification already processed
    VerificationAlreadyProcessed = 16,

    /// Cannot review own project
    CannotReviewOwnProject = 17,

    /// Fee configuration not set
    FeeConfigNotSet = 18,

    /// Treasury address not set
    TreasuryNotSet = 19,

    /// Review already deleted
    ReviewAlreadyDeleted = 20,
}