use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ContractError {
    AlreadyInitialized = 0,
    NotInitialized = 1,
    NotAuthorized = 2,
    ProjectNotFound = 3,
    ProjectAlreadyExists = 4,
    InvalidSlug = 5,
    MaxProjectsExceeded = 6,
    MaxMaintainersExceeded = 7,
    MaintainerNotFound = 8,
    InvalidCategory = 9,
    InvalidBountyUrl = 10,
    InvalidBountyCid = 11,
}