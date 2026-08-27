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
    pub license: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<Map<String, String>>,
    pub launch_timestamp: Option<u64>,
    pub bounty_url: Option<String>,
    pub repository_url: Option<String>,
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
    pub license: Option<Option<String>>,
    pub logo_cid: Option<Option<String>>,
    pub metadata_cid: Option<Option<String>>,
    pub tags: Option<Option<Vec<String>>>,
    pub social_links: Option<Option<Map<String, String>>>,
    pub launch_timestamp: Option<Option<u64>>,
    pub bounty_url: Option<Option<String>>,
    pub repository_url: Option<Option<String>>,
    // NOTE: lifecycle status is deliberately not updatable here. It has its own
    // entry point, `set_project_lifecycle_status`, which emits a dedicated
    // event. A `lifecycle_status` field previously sat here but was never read
    // by `update_project`, so it silently did nothing — while its
    // `Option<unit-enum>` type broke every `testutils` build (soroban-sdk 22
    // generates only `TryFrom<T> for ScVal` on unit enums, and `Option<T>`
    // needs the by-value `From`). That is why `cargo build` passed while
    // `cargo test` could not compile at all.
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
    Revised,
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

/// Snapshot of a review before an edit. Stored in ascending revision_index order (0 = first edit).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewRevision {
    pub revision_index: u32,
    pub rating: u32,
    pub content_cid: Option<String>,
    pub revised_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewRevisionEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub revision_index: u32,
    pub previous_rating: u32,
    pub previous_content_cid: Option<String>,
    pub new_rating: u32,
    pub new_content_cid: Option<String>,
    pub timestamp: u64,
}

/// Shared three-state status for all claim workflows (ownership + contract-address).
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimStatus {
    Pending,
    Approved,
    Rejected,
}

/// Distinguishes claim workflow kinds that share [`ClaimStatus`] transitions.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimKind {
    /// Claim ownership of a claimable project.
    Ownership,
    /// Claim a contract address for a project.
    ContractAddress,
}

impl ClaimStatus {
    /// Shared pending→approved / pending→rejected guard used by every claim kind.
    pub fn require_pending(self) -> Result<(), crate::errors::ContractError> {
        if self != Self::Pending {
            Err(crate::errors::ContractError::InvalidStatus)
        } else {
            Ok(())
        }
    }

    /// Transition Pending → Approved.
    pub fn transition_to_approved(&mut self) -> Result<(), crate::errors::ContractError> {
        self.require_pending()?;
        *self = Self::Approved;
        Ok(())
    }

    /// Transition Pending → Rejected.
    pub fn transition_to_rejected(&mut self) -> Result<(), crate::errors::ContractError> {
        self.require_pending()?;
        *self = Self::Rejected;
        Ok(())
    }
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
pub struct ContractClaimRequest {
    pub project_id: u64,
    pub contract_address: String,
    pub claimant: Address,
    pub proof_cid: String,
    pub status: ClaimStatus,
    pub created_at: u64,
    /// Unix timestamp (seconds) after which this pending claim is considered expired.
    /// A value of 0 means no expiry (legacy). New claims always set this to
    /// `created_at + CLAIM_EXPIRY_SECONDS`.
    pub expires_at: u64,
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
    pub license: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub verification_status: VerificationStatus,
    pub current_verification_id: Option<u64>,
    pub archived: bool,
    pub claimable: bool,
    pub lifecycle_status: ProjectLifecycleStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<Map<String, String>>,
    pub launch_timestamp: Option<u64>,
    pub maintainers: Option<Vec<Address>>,
    pub bounty_url: Option<String>,
    pub repository_url: Option<String>,
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Rejected,
}

/// Project lifecycle status for managing project activity state.
/// Allows project owners to signal project maturity, stability, and maintenance status.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectLifecycleStatus {
    /// Active development - project is regularly maintained
    Active,
    /// Beta/experimental - not yet stable for production use
    Beta,
    /// Paused - temporarily not maintained
    Paused,
    /// Deprecated - no longer recommended for new use
    Deprecated,
    /// Sunset - officially discontinued
    Sunset,
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

/// A refund owed to a project owner after their verification request was
/// rejected (issue #472).
///
/// Rejection records the debt rather than transferring immediately: paying out
/// requires the treasury's authorization, and the rejecting admin cannot be
/// expected to hold the treasury key. The payer (or an admin acting for them)
/// settles it later via `claim_fee_refund`, and that transaction carries the
/// treasury signature.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeRefundRecord {
    /// Project whose verification fee is being refunded.
    pub project_id: u64,
    /// Verification request that was rejected.
    pub request_id: u64,
    /// Address that paid the fee and is owed the refund.
    pub payer: Address,
    /// Amount owed, in the smallest unit of `token`.
    pub amount: u128,
    /// Token the fee was paid in. `None` when the fee was configured as free.
    pub token: Option<Address>,
    /// Ledger timestamp at which the refund became claimable.
    pub created_at: u64,
    /// Ledger timestamp of the payout, or `None` while still outstanding.
    pub claimed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigHistoryEntry {
    pub admin: Address,
    pub old_token: Option<Address>,
    pub old_verification_fee: Option<u128>,
    pub old_registration_fee: Option<u128>,
    pub old_treasury: Option<Address>,
    pub token: Option<Address>,
    pub verification_fee: u128,
    pub registration_fee: u128,
    pub treasury: Address,
    pub timestamp: u64,
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
    VerificationAssigned,
    ReservedNameAdded,
    ReservedNameRemoved,
    /// Admin toggled the global pause flag on (`true` was the new value).
    ContractPaused,
    /// Admin toggled the global pause flag off (`false` was the new value).
    ContractResumed,
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

// ── Admin Timelock ───────────────────────────────────────────────────────────

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
    pub approvals: Map<Address, bool>,
    pub status: ProposalStatus,
    pub created_at: u64,
    /// Optional expiry timestamp (Unix seconds). When non-zero, `execute_proposal`
    /// will reject the proposal if the current ledger time is at or past this value.
    /// Zero means no expiry (legacy / always executable once approved).
    pub expires_at: u64,
}

/// Tombstone stored when a review is deleted so indexers can distinguish
/// deleted reviews from reviews that never existed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewTombstone {
    pub project_id: u64,
    pub reviewer: Address,
    pub deleted_at: u64,
}

/// Optional anti-sybil review eligibility constraints.
///
/// When all constraints are zero/false (default), any address may review
/// any project without restriction — preserving full backward compatibility.
///
/// Admins may relax or tighten these knobs via `set_review_eligibility_config`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewEligibilityConfig {
    /// Minimum seconds that must have elapsed since the reviewer's first
    /// interaction with the contract (e.g. first review, project registration,
    /// endorsement, follow, or bookmark). Zero = no age check.
    pub min_reviewer_age_seconds: u64,
    /// If true, the reviewer must have previously endorsed the project
    /// (`EndorsementRegistry::has_endorsed`) before submitting a review.
    pub require_endorsement: bool,
    /// Fee amount (in the configured fee token) required to submit a review.
    /// Zero = no fee required. When non-zero, the caller must have paid this
    /// amount to the treasury before submitting the review.
    pub review_fee: u128,
}

/// Sort order retained for `list_reviews_sorted` ABI compatibility.
/// Sorting is performed client-side.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewSortMode {
    /// Newest reviews first (highest created_at).
    Newest,
    /// Oldest reviews first (lowest created_at).
    Oldest,
    /// Highest rating first.
    RatingHigh,
    /// Lowest rating first.
    RatingLow,
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

/// Project changelog entry for publishing update notes or release history.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangelogEntry {
    /// Unique identifier for the changelog entry
    pub id: u64,
    /// Project ID this changelog belongs to
    pub project_id: u64,
    /// IPFS CID containing the changelog content
    pub cid: String,
    /// Timestamp when the changelog was added
    pub created_at: u64,
    /// Optional description/title for the changelog entry
    pub description: Option<String>,
    /// Optional semantic version string for this release (e.g. "1.2.3").
    /// Allows indexers to correlate changelog entries with project releases.
    pub version: Option<String>,
    /// Optional IPFS CID pointing to a structured release-notes document.
    /// Complements `cid` when separate machine-readable release metadata is needed.
    pub changelog_cid: Option<String>,
}

/// Changelog sort order for paginated reads
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangelogSortMode {
    /// Newest changelog entries first (highest created_at)
    Newest,
    /// Oldest changelog entries first (lowest created_at)
    Oldest,
}

// ── Contract configuration view (returned by `get_config`) ──────────────────

/// User-facing limits surfaced through `get_config`. Only the most relevant
/// limits for frontend validation are exposed — internal string-length
/// bounds (e.g. `MAX_WEBSITE_LEN`) are intentionally omitted to keep the
/// response shape stable.
///
/// **Stability:** Adding fields here is backwards-compatible. Removing or
/// renaming a field is a breaking change and requires a `CONTRACT_VERSION`
/// bump in `constants.rs`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractLimits {
    /// Maximum items per paginated list call (`MAX_PAGE_LIMIT`).
    pub max_page_limit: u32,
    /// Maximum projects a single owner may register (`MAX_PROJECTS_PER_USER`).
    pub max_projects_per_user: u32,
    /// Maximum reviewers indexed per project (`MAX_REVIEWS_PER_PROJECT`).
    pub max_reviews_per_project: u32,
    /// Maximum project name length in bytes (`MAX_NAME_LEN`).
    pub max_name_len: u32,
    /// Maximum project description length in bytes (`MAX_DESCRIPTION_LEN`).
    pub max_description_len: u32,
    /// Verification validity period in seconds (`VERIFICATION_VALIDITY_PERIOD`).
    pub verification_validity_period: u64,
}

/// Aggregated, read-only contract configuration snapshot. Frontends and
/// indexers call `get_config` to read this in one round-trip instead of
/// walking the individual getters (`get_fee_config`, `get_admin_count`,
/// …).
///
/// **Stability:** The shape of this struct is part of the public contract
/// interface. New fields may be appended at the end; never reorder,
/// rename, or remove existing fields without bumping `CONTRACT_VERSION`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractConfigView {
    /// Semantic version of the contract (`CONTRACT_VERSION`).
    pub version: String,
    /// Number of admin addresses currently registered.
    pub admin_count: u32,
    /// Approval threshold for multi-admin proposal workflows
    /// (`get_admin_approval_threshold`).
    pub admin_approval_threshold: u32,
    /// Global pause flag. Read by frontends to disable mutating UX. Set
    /// by admins via `set_pause`.
    pub paused: bool,
    /// Treasury address that receives fees. `None` until `set_fee` is
    /// called for the first time.
    pub treasury: Option<Address>,
    /// Current fee configuration (token + verification + registration fee).
    pub fees: FeeConfig,
    /// User-facing limits (see `ContractLimits` doc for stability rules).
    pub limits: ContractLimits,
}
