//! Contract error codes and revert messages.
//! All errors are descriptive and used for validation, authorization, and invalid input failures.

use soroban_sdk::contracterror;

/// Contract error codes. Values are stable for client bindings and indexing.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    // ---- Project registry (1xx) ----
    /// Project not found.
    ProjectNotFound = 101,
    /// Caller is not the project owner; unauthorized update.
    NotProjectOwner = 102,
    /// Project name is empty or invalid.
    InvalidProjectName = 103,
    /// Project description is empty or invalid.
    InvalidProjectDescription = 104,
    /// Project category is empty or invalid.
    InvalidProjectCategory = 105,
    /// String length exceeds maximum allowed (name, description, category, website, CIDs).
    StringLengthExceeded = 106,
    /// User has reached the maximum number of projects they can register.
    MaxProjectsPerUserExceeded = 107,
    /// Project ID is invalid (e.g. zero).
    InvalidProjectId = 108,

    // ---- Review registry (2xx) ----
    /// Rating must be between 1 and 5 (inclusive).
    InvalidRating = 201,
    /// Reviewer has already submitted a review for this project; use update instead.
    DuplicateReview = 202,
    /// No review found for this project and reviewer.
    ReviewNotFound = 203,
    /// Only the original reviewer can update or delete the review.
    NotReviewAuthor = 204,
    /// Cannot compute aggregates when there are zero reviews (e.g. average rating).
    ZeroReviews = 205,
    /// CID is present but invalid: wrong prefix or too short.
    /// Valid CIDs start with "Qm" (CIDv0, 46 chars) or "bafy" (CIDv1).
    InvalidCid = 206,

    // ---- Verification registry (3xx) ----
    /// Verification record not found for this project.
    VerificationNotFound = 301,
    /// Caller is not the project owner; only owner can request verification.
    NotProjectOwnerForVerification = 302,
    /// Verification fee has not been paid for this project.
    FeeNotPaid = 303,
    /// Evidence CID is empty or invalid.
    InvalidEvidenceCid = 304,
    /// Only admin or authorized verifier can approve or reject.
    UnauthorizedVerifier = 305,
    /// Verification is not in Pending state (already approved or rejected).
    VerificationNotPending = 306,

    // ---- Fee manager (4xx) ----
    /// Only admin can set fee configuration.
    UnauthorizedAdmin = 401,
    /// Fee amount must be greater than zero when fee is enabled.
    InvalidFeeAmount = 402,
    /// Treasury address is invalid.
    InvalidTreasury = 403,
    /// Payment failed (transfer to treasury failed).
    PaymentFailed = 404,
    /// Fee configuration not set (token/amount/treasury).
    FeeNotConfigured = 405,
}
