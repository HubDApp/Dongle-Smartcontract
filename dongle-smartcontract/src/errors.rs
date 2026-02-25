use soroban_sdk::contracterror;

/// Error types for the Dongle smart contract
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
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
    /// Verification record not found
    VerificationNotFound = 6,
    /// Invalid verification status transition
    InvalidStatusTransition = 7,
    /// Only admin can perform this action
    AdminOnly = 8,
    /// Invalid fee amount
    InvalidFeeAmount = 9,
    /// Insufficient fee paid
    InsufficientFee = 10,
    /// Invalid project data - missing required fields
    InvalidProjectData = 11,
    /// Project name too long
    ProjectNameTooLong = 12,
    /// Project description too long
    ProjectDescriptionTooLong = 13,
    /// Invalid project category
    InvalidProjectCategory = 14,
    /// Verification already processed
    VerificationAlreadyProcessed = 15,
    /// Cannot review own project
    CannotReviewOwnProject = 16,
    /// Fee configuration not set
    FeeConfigNotSet = 17,
    /// Treasury address not set
    TreasuryNotSet = 18,
    /// User has already reviewed this project
    AlreadyReviewed = 19,
    /// Insufficient balance in treasury
    InsufficientBalance = 20,
    /// Invalid withdrawal amount
    InvalidAmount = 21,
    /// Fee not configured for this token
    FeeNotConfigured = 22,
    /// Treasury address not set
    TreasuryAddressNotSet = 23,
    AlreadyReviewed = 19, // I added your error here with a new unique ID
    /// Review is already deleted
    ReviewAlreadyDeleted = 20,
}