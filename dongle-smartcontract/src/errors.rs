//! Contract error codes and revert messages.
//! All errors are descriptive and used for validation, authorization, and invalid input failures.

use soroban_sdk::contracterror;

/// Contract error codes. Values are stable for client bindings and indexing.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    ProjectNotFound = 1,
    Unauthorized = 2,
    ProjectAlreadyExists = 3,
    InvalidRating = 4,
    ReviewNotFound = 5,
    VerificationNotFound = 6,
    InvalidStatusTransition = 7,
    AdminOnly = 8,
    InvalidFeeAmount = 9,
    InsufficientFee = 10,
    InvalidProjectData = 11,
    ProjectNameTooLong = 12,
    ProjectDescriptionTooLong = 13,
    InvalidProjectCategory = 14,
    VerificationAlreadyProcessed = 15,
    CannotReviewOwnProject = 16,
    FeeConfigNotSet = 17,
    TreasuryNotSet = 18,
    NotReviewer = 19,
}
