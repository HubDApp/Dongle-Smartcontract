use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Review {
    pub reviewer: Address,
    pub rating: u8,
    pub comment_cid: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct VerificationRecord {
    pub status: VerificationStatus,
    pub evidence_cid: String,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    Project(u64), // project_id
    ProjectNameHash(soroban_sdk::BytesN<32>), // hashed name without spaces and lowercase
    ProjectHash(soroban_sdk::BytesN<32>), // full metadata hash
    ProjectCounter,
}
