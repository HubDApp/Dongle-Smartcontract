use soroban_sdk::{contracttype, Address, String};

/// Represents a project registered in the Dongle ecosystem
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub updated_at: u64,
    pub is_verified: bool,
}

/// Represents a review for a project
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32, // Changed from u8 to u32 for upstream consistency
    pub comment_cid: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Status of a verification request
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    Pending,
    Approved,
    Rejected,
}

/// Represents a verification record for a project
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRecord {
    pub project_id: u64,
    pub requester: Address,
    pub status: VerificationStatus,
    pub evidence_cid: String,
    pub verifier: Option<Address>,
    pub requested_at: u64,
    pub processed_at: Option<u64>,
}

/// Storage keys for the contract data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Project(u64),
    /// ProjectID and Reviewer address used for the compound key
    ProjectReview(u64, Address), 
    Verification(u64),
    NextProjectId,
    Admin(Address),
    FeeConfig,
    Treasury,
}

/// Fee configuration for contract operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub token: Option<Address>,
    pub verification_fee: u128,
    pub registration_fee: u128,
}