//! Contract error codes and revert messages.
//! All errors are descriptive and used for validation, authorization, and invalid input failures.

use soroban_sdk::contracterror;

/// Contract error codes. Values are stable for client bindings and indexing.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Project not found in storage.
    ProjectNotFound = 1,
    /// Caller is not authorized for this action.
    Unauthorized = 2,
    /// Project ID already exists.
    ProjectAlreadyExists = 3,
    /// Rating must be between 1 and 5.
    InvalidRating = 4,
    /// Review not found for this project/reviewer.
    ReviewNotFound = 5,
    /// Verification record not found.
    VerificationNotFound = 6,
    /// Invalid state transition for verification.
    InvalidStatusTransition = 7,
    /// Restricted to contract admins.
    AdminOnly = 8,
    /// Fee amount must be greater than zero.
    InvalidFeeAmount = 9,
    /// Payment attached is less than required fee.
    InsufficientFee = 10,
    /// Project data is malformed or invalid.
    InvalidProjectData = 11,
    /// Project name exceeds maximum length.
    ProjectNameTooLong = 12,
    /// Project description exceeds maximum length.
    ProjectDescriptionTooLong = 13,
    /// Category is not recognized.
    InvalidProjectCategory = 14,
    /// Verification request has already been handled.
    VerificationAlreadyProcessed = 15,
    /// Owners cannot review their own projects.
    CannotReviewOwnProject = 16,
    /// Fee configuration is missing.
    FeeConfigNotSet = 17,
    /// Treasury address is missing.
    TreasuryNotSet = 18,
    /// Action restricted to authorized reviewers.
    NotReviewer = 19,
}
