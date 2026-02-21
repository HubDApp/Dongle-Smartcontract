use soroban_sdk::{contracttype, Address, String};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Project {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub verification_status: VerificationStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,
    pub comment_cid: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct VerificationRecord {
    pub project_id: u64,
    pub requester: Address,
    pub verifier: Option<Address>,
    pub status: VerificationStatus,
    pub evidence_cid: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DataKey {
    Project(u64),
    Review(u64, Address),
    Verification(u64),
    NextProjectId,
    Admin(Address),
    FeeConfig,
    Treasury,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct FeeConfig {
    pub token: Option<Address>,
    pub amount: u128,
    pub treasury: Address,
}
