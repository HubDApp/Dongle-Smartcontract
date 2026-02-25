//! Contract error codes and revert messages.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    // ── Core project errors (1–14) ─────────────────────────────────────────
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
    /// Returned when category is not one of: DeFi, NFT, Gaming, DAO, Tools.
    InvalidProjectCategory = 14,

    // ── Verification / review errors (15–20) ──────────────────────────────
    VerificationAlreadyProcessed = 15,
    CannotReviewOwnProject = 16,
    FeeConfigNotSet = 17,
    TreasuryNotSet = 18,
    NotReviewer = 19,
    VerificationNotPending = 20,

    // ── Fee manager errors (21–22) ────────────────────────────────────────
    UnauthorizedAdmin = 21,
    FeeNotConfigured = 22,

    // ── String / validation errors (23) ──────────────────────────────────
    StringLengthExceeded = 23,

    // ── Review errors (24–25) ─────────────────────────────────────────────
    DuplicateReview = 24,
    NotReviewAuthor = 25,

    // ── Project field errors (26–27) ──────────────────────────────────────
    InvalidProjectName = 26,
    InvalidProjectDescription = 27,

    // ── Verification submission errors (28–32) ────────────────────────────
    InvalidProjectId = 28,
    InvalidEvidenceCid = 29,
    NotProjectOwnerForVerification = 30,
    FeeNotPaid = 31,
    UnauthorizedVerifier = 32,
}

/// Type alias so that modules importing `crate::errors::Error` continue to compile.
pub type Error = ContractError;
