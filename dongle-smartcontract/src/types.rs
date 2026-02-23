use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub registered_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,   // u8 not supported by contracttype, use u32
    pub comment_cid: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct VerificationRecord {
    pub project_id: u64,
    pub requester: Address,
    pub evidence_cid: String,
    pub status: VerificationStatus,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Rejected,
}