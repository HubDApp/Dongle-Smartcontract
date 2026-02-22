use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    InvalidRating = 1,           // Rating not in range 1-5
    ReviewNotFound = 2,          // Attempting to update/delete non-existent review
    ReviewAlreadyExists = 3,     // Attempting to add duplicate review
    ProjectNotFound = 4,         // Project doesn't exist
    UnauthorizedReviewer = 5,    // Caller is not the original reviewer
}
