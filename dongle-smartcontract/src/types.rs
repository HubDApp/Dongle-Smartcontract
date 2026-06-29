use soroban_sdk::{contracttype, Address, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegistrationParams {
    pub owner: Address,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<Vec<String>>,
    pub launch_timestamp: Option<u64>,
    pub license: Option<String>,
    pub bounty_url: Option<String>,
    pub bounty_cid: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdateParams {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub website: Option<Option<String>>,
    pub logo_cid: Option<Option<String>>,
    pub metadata_cid: Option<Option<String>>,
    pub tags: Option<Option<Vec<String>>>,
    pub social_links: Option<Option<Vec<String>>>,
    pub launch_timestamp: Option<Option<u64>>,
    pub license: Option<Option<String>>,
    pub bounty_url: Option<Option<String>>,
    pub bounty_cid: Option<Option<String>>,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractClaimStatus {
    Pending,
    Approved,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaimRequest {
    pub project_id: u64,
    pub contract_address: String,
    pub claimant: Address,
    pub proof_cid: String,
    pub status: ContractClaimStatus,
    pub created_at: u64,
}


#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<Vec<String>>,
    pub launch_timestamp: Option<u64>,
    pub license: Option<String>,
    pub bounty_url: Option<String>,
    pub security_contact: Option<String>,
    pub security_contact_proof_cid: Option<String>,
    pub security_contact_verified: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityContactStatus {
    pub contact: Option<String>,
    pub proof_cid: Option<String>,
    pub verified: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReport {
    pub project_id: u64,
    pub reporter: Address,
    pub reason_cid: String,
    pub timestamp: u64,
}

#[contracttype]
pub enum DataKey {
    Project(u64),
    ProjectCount,
    OwnerProjects(Address),
    Review(u64, Address),
    UserReviews(Address),
    Verification(u64),
    NextProjectId,
    Admin(Address),
    FeeConfig,
    Treasury,
    ProjectStats(u64),
    FeePaidForProject(u64),
    ContractClaim(u64, String),
    ProjectContracts(u64),
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRecord {
    pub request_id: u64,
    pub project_id: u64,
    pub requester: Address,
    pub status: VerificationStatus,
    pub evidence_cid: String,
    pub requested_at: u64,
    pub decided_at: u64,
    pub fee_amount: u128,
    pub revoke_reason: Option<String>,
    /// Unix timestamp when verification expires (0 = no expiry)
    pub expires_at: u64,
    /// Unix timestamp when verification was last renewed
    pub last_renewed_at: u64,
    /// Admin assigned to review this verification request
    pub assigned_admin: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewalRecord {
    pub project_id: u64,
    pub requester: Address,
    pub status: VerificationStatus,
    pub evidence_cid: String,
    pub timestamp: u64,
    pub fee_amount: u128,
    /// Unix timestamp when the renewed verification expires
    pub expires_at: u64,
}

/// Fee configuration for contract operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub token: Option<Address>,
    pub verification_fee: u128,
    pub registration_fee: u128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaymentRecord {
    pub paid_at: u64,
    pub payer: Address,
    pub amount: u128,
    pub token: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeRefundRecord {
    pub request_id: u64,
    pub project_id: u64,
    pub payer: Address,
    pub token: Option<Address>,
    pub amount: u128,
    pub refunded: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigHistoryEntry {
    pub admin: Address,
    pub token: Option<Address>,
    pub verification_fee: u128,
    pub registration_fee: u128,
    pub treasury: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Default)]
pub struct ProjectAggregate {
    pub total_rating: u64,
    pub review_count: u64,
}

// ── Project dependencies ─────────────────────────────────────────────────────

/// External dependency reference can point to an internal project id,
/// an external IPFS CID, an external URL, or a Stellar contract address.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyRef {
    /// Another project inside this contract.
    pub project_id: Option<u64>,
    /// External content-addressed reference (e.g. ipfs cid).
    pub external_cid: Option<String>,
    /// External URL reference (http/https).
    pub external_url: Option<String>,
    /// External Stellar contract address (56-char Strkey, starts with 'C').
    pub external_contract: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDependency {
    /// The unique reference identifying the dependency.
    pub reference: DependencyRef,
    /// Optional free-form label (e.g. "oracle", "token", "protocol").
    pub label: Option<String>,
    /// Optional metadata CID describing the dependency.
    pub metadata_cid: Option<String>,
    /// Unix timestamp (seconds) when the dependency was added.
    pub added_at: u64,
    /// Unix timestamp (seconds) when the dependency was last updated.
    pub updated_at: u64,
}

/// Emitted when a project's featured status changes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeaturedProjectEvent {
    pub project_id: u64,
    pub featured: bool,
    pub admin: Address,
    pub timestamp: u64,
}

/// A curated collection of projects, managed by admins.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collection {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub maintainers: Option<Vec<Address>>,
    pub verification_status: Option<u32>,
    // ... other fields
}

/// Sort order for `list_projects_sorted`. Sorting is performed on-chain in-memory.
/// To prevent unbounded loops, this fetches up to a maximum limit.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectSortMode {
    /// Newest projects first (highest created_at).
    Newest,
    /// Oldest projects first (lowest created_at).
    Oldest,
    /// Highest rated first.
    HighestRated,
    /// Most reviewed first.
    MostReviewed,
}
