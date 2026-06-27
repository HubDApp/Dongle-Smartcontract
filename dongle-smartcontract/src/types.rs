use soroban_sdk::{contracttype, Address, Map, String, Vec};

#[contracttype]
#[derive(Clone, Debug)]
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
    pub social_links: Option<Map<String, String>>,
    pub launch_timestamp: Option<u64>,
    pub bounty_url: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ProjectUpdateParams {
    pub project_id: u64,
    pub caller: Address,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub website: Option<Option<String>>,
    pub logo_cid: Option<Option<String>>,
    pub metadata_cid: Option<Option<String>>,
    pub tags: Option<Option<Vec<String>>>,
    pub social_links: Option<Option<Map<String, String>>>,
    pub launch_timestamp: Option<Option<u64>>,
    pub bounty_url: Option<Option<String>>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectStats {
    pub rating_sum: u64,
    pub review_count: u32,
    pub average_rating: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,
    /// Canonical content CID - replaces the redundant ipfs_cid/comment_cid pair
    pub content_cid: Option<String>,
    pub owner_response: Option<String>,

    /// Unix timestamp (seconds) when the review was first submitted.
    pub created_at: u64,

    /// Unix timestamp (seconds) of the most recent modification to this review.
    pub updated_at: u64,

    /// Whether the review is hidden by moderation.
    pub hidden: bool,

    /// Number of times this review has been reported.
    pub report_count: u32,
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
    /// Canonical content CID - consolidates the review content
    pub content_cid: Option<String>,
    pub owner_response: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimStatus {
    Pending,
    Approved,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRequest {
    pub id: u64,
    pub project_id: u64,
    pub claimant: Address,
    pub proof_cid: String,
    pub status: ClaimStatus,
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
    pub verification_status: VerificationStatus,
    pub current_verification_id: Option<u64>,
    pub archived: bool,
    pub claimable: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<Map<String, String>>,
    pub launch_timestamp: Option<u64>,
    pub maintainers: Option<Vec<Address>>,
    pub bounty_url: Option<String>,
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
}

/// Parameters for creating a new collection (admin-only).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateCollectionParams {
    pub name: String,
    pub description: String,
}

/// Types of admin actions recorded in the admin action log.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminActionType {
    AdminAdded,
    AdminRemoved,
    VerificationApproved,
    VerificationRejected,
    VerificationRevoked,
    VerificationRenewalApproved,
    VerificationRenewalRejected,
    FeeChanged,
    MinProjectAgeSet,
    ReviewHidden,
    ReviewRestored,
    ReviewDeletedByAdmin,
    ProjectReportsCleared,
    VerificationHistoryCleared,
    RenewalHistoryCleared,
    CollectionCreated,
    CollectionUpdated,
    CollectionDeleted,
    ProjectAddedToCollection,
    ProjectRemovedFromCollection,
    ProjectFeatured,
    ProjectUnfeatured,
    DuplicateDisputeResolved,
    DuplicateDisputeRejected,
    VerificationDurationSet,
    ThresholdChanged,
    FeeRefunded,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    Pending,
    Rejected,
    Resolved,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateDispute {
    pub id: u64,
    pub project_id: u64,
    pub original_project_id: u64,
    pub creator: Address,
    pub evidence_cid: String,
    pub status: DisputeStatus,
    pub created_at: u64,
    pub resolved_at: u64,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisputeResolutionAction {
    Reject,
    ArchiveProject(u64),
    LinkDuplicates,
}

/// A single entry in the admin action log.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminActionEntry {
    pub id: u64,
    pub admin: Address,
    pub action_type: AdminActionType,
    pub target_id: Option<u64>,
    pub target_address: Option<Address>,
    pub timestamp: u64,
    pub reason_cid: Option<String>,
}

// ── Admin Timelock ──────────────────────────────────────────────────────────

/// A scheduled action in the admin timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockAction {
    pub id: u64,
    pub admin: Address,
    pub action_type: AdminActionType,
    pub execution_timestamp: u64,
    pub executed: bool,
    pub cancelled: bool,
    pub created_at: u64,
}

/// Parameters for a scheduled fee change via timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockFeeParams {
    pub token: Option<Address>,
    pub verification_fee: u128,
    pub registration_fee: u128,
    pub treasury: Address,
}

/// Parameters for a scheduled admin addition via timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockAdminAddParams {
    pub new_admin: Address,
}

/// Parameters for a scheduled admin removal via timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockAdminRemoveParams {
    pub admin_to_remove: Address,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Executed,
    Rejected,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalPayload {
    AddAdmin(Address),
    RemoveAdmin(Address),
    SetFee(Option<Address>, u128, u128, Address),
    SetThreshold(u32),
    ApproveVerification(u64),
    RejectVerification(u64),
    RevokeVerification(u64, String),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminProposal {
    pub id: u64,
    pub proposer: Address,
    pub action_type: AdminActionType,
    pub payload_hash: soroban_sdk::BytesN<32>,
    pub payload: ProposalPayload,
    pub approvals: Vec<Address>,
    pub status: ProposalStatus,
    pub created_at: u64,
}
