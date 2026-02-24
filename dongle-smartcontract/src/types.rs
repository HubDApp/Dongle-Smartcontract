use soroban_sdk::{Address, String, contracttype};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,
    pub comment_cid: Option<String>,
    pub is_deleted: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ProjectStats {
    pub rating_sum: u64,
    pub review_count: u32,
    pub average_rating: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewAction {
    Submitted,
    Updated,
    Deleted,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewEventData {
    pub project_id: u64,
    pub reviewer: Address,
    pub action: ReviewAction,
    pub timestamp: u64,
    pub comment_cid: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: u64,
}

#[contracttype]
pub enum DataKey {
    Project(u64),
    Review(u64, Address),
    Verification(u64),
    NextProjectId,
    Admin(Address),
    FeeConfig,
    Treasury,
    ProjectStats(u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRecord {
    pub status: VerificationStatus,
}

/// Fee configuration for contract operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub token: Option<Address>,
    pub verification_fee: u128,
    pub registration_fee: u128,
}