use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    OnlyAdmin = 3,
    ProjectNotFound = 4,
    NotProjectOwner = 5,
    SlugAlreadyExists = 6,
    InvalidSlug = 7,
    MaxProjectsExceeded = 8,
    MaxReviewsPerUser = 9,
    MaxReviewsPerProject = 10,
    ReviewNotFound = 11,
    AlreadyReviewed = 12,
    InvalidCategory = 13,
    InvalidUrl = 14,
    InvalidCid = 15,
    InvalidBountyUrl = 16,
    InvalidBountyCid = 17,
    InvalidWebsite = 18,
    InvalidLogo = 19,
    InvalidMetadata = 20,
    InvalidTags = 21,
    InvalidSocialLinks = 22,
    InvalidLauchTimestamp = 23,
    InvalidLicense = 24,
    AlreadyMaintainer = 25,
    NotMaintainer = 26,
    OnlyMaintainerOrOwner = 27,
    InvalidMaintainer = 28,
    CantRemoveSelf = 29,
    IndexOutOfBounds = 30,
    NotInIndex = 31,

    // Project registration and updates
    ProjectAlreadyExists = 32,
    InvalidProjectName = 33,
    ProjectNameTooLong = 34,
    InvalidProjectDesc = 35,
    ProjectDescTooLong = 36,
    InvalidProjectData = 37,
    InvalidProjectSlug = 38,
    InvalidProjectSlugLen = 39,
    InvalidInput = 40,

    // CID-specific
    InvalidLogoCid = 41,
    InvalidMetaCid = 42,

    // Authorization / access control
    Unauthorized = 43,
    AdminOnly = 44,
    AdminNotFound = 45,

    // Verification workflow
    VerificationNotFound = 46,
    VerificationNotPend = 47,
    InvalidStatus = 48,
    ProjectTooYoung = 49,
    VerifiedFieldFrozen = 50,

    // Archive / reactivation
    AlreadyArchived = 51,
    ProjectNotArchived = 52,

    // Ownership transfer
    TransferNotFound = 53,
    NotTransferRecip = 54,

    // Reserved names
    ReservedName = 55,

    // Fee
    FeeMissing = 56,
    FeeInvalid = 57,
    FeeAlreadyPaid = 58,

    // Security contact
    SecurityContactInvalid = 59,

    // Normalized name duplicate
    DuplicateProjectName = 60,
}
