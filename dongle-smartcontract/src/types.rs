use soroban_sdk::{contracttype, Address, String};

/// Represents a project registered in the Dongle ecosystem
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// Unique identifier for the project
    pub id: u64,
    /// Address of the project owner
    pub owner: Address,
    /// Name of the project
    pub name: String,
    /// Description of the project
    pub description: String,
    /// Category of the project (e.g., DeFi, Gaming, etc.)
    pub category: String,
    /// Optional website URL
    pub website: Option<String>,
    /// Optional IPFS CID for project logo
    pub logo_cid: Option<String>,
    /// Optional IPFS CID for additional metadata
    pub metadata_cid: Option<String>,
    /// Timestamp when the project was registered
    pub registered_at: u64,
    /// Timestamp when the project was last updated
    pub updated_at: u64,
    /// Whether the project is verified
    pub is_verified: bool,
}

/// Represents a review for a project
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    /// ID of the project being reviewed
    pub project_id: u64,
    /// Address of the reviewer
    pub reviewer: Address,
    /// Rating given (1-5 stars)
    pub rating: u32,
    /// Optional IPFS CID for review comment
    pub comment_cid: Option<String>,
    /// Timestamp when the review was created
    pub created_at: u64,
    /// Timestamp when the review was last updated
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
    /// ID of the project being verified
    pub project_id: u64,
    /// Address of the requester
    pub requester: Address,
    /// Current status of the verification
    pub status: VerificationStatus,
    /// IPFS CID for evidence/documentation
    pub evidence_cid: String,
    /// Optional address of the verifier who approved/rejected
    pub verifier: Option<Address>,
    /// Timestamp when verification was requested
    pub requested_at: u64,
    /// Timestamp when verification was processed (if applicable)
    pub processed_at: Option<u64>,
}

/// Storage keys for the contract data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Key for storing projects: DataKey::Project(project_id)
    Project(u64),
    /// Key for storing reviews: DataKey::Review(project_id, reviewer_address)
    Review(u64, Address),
    /// Key for storing verification records: DataKey::Verification(project_id)
    Verification(u64),
    /// Key for storing the next project ID counter
    NextProjectId,
    /// Key for storing admin addresses
    Admin(Address),
    /// Key for storing fee configuration
    FeeConfig,
    /// Key for storing treasury address
    Treasury,
}

/// Fee configuration for contract operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    /// Optional token address for fee payment (None means native XLM)
    pub token: Option<Address>,
    /// Amount to be paid for verification requests
    pub verification_fee: u128,
    /// Amount to be paid for project registration (if any)
    pub registration_fee: u128,
}