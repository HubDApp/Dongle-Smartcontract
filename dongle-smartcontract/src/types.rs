use soroban_sdk::{Address, String, contracttype};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,
    pub comment_cid: Option<String>,
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
