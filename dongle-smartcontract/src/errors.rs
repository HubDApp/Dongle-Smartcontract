use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    ProjectNotFound = 1,
    Unauthorized = 2,
    ProjectAlreadyExists = 3,
    InvalidRating = 4,
    ReviewNotFound = 5,
    ReviewAlreadyExists = 6,
    VerificationNotFound = 7,
    InvalidStatusTransition = 8,
    AdminOnly = 9,
    InvalidFeeAmount = 10,
    InsufficientFee = 11,
    InvalidProjectData = 12,
    ProjectNameTooLong = 13,
    ProjectDescriptionTooLong = 14,
    InvalidProjectCategory = 15,
    VerificationAlreadyProcessed = 16,
    CannotReviewOwnProject = 17,
    FeeConfigNotSet = 18,
    TreasuryNotSet = 19,
    NotFound = 20,
    AlreadyExists = 21,
}

