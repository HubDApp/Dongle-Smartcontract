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

    /// Unix timestamp (seconds) of the most recent reviewer update.
    /// Zero means the review has not been updated since submission.
    pub last_updated_at: u64,

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

/// Audit trail event emitted when a reviewer updates their review rating or content.
/// This provides transparency for all rating changes, allowing tracking of:
/// - When a rating was changed (timestamp)
/// - Who changed it (reviewer)
/// - What the previous rating was (previous_rating)
/// - What the new rating is (new_rating)
/// - The revision index for ordering changes (revision_index)
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
///
/// ## State Machine
///
/// ```text
///         submit_claim_request
///              │
///              ▼
///           Pending  ──── approve ────► Approved  (terminal)
///              │
///              └──── reject ────────► Rejected  (terminal)
/// ```
///
/// ### Valid transitions
///
/// | From    | To       | Triggered by                      |
/// |---------|----------|-----------------------------------|
/// | Pending | Approved | admin calls `approve_claim_request` |
/// | Pending | Rejected | admin calls `reject_claim_request`  |
///
/// ### Terminal states
///
/// `Approved` and `Rejected` are terminal — once a claim reaches either state
/// no further transition is permitted.  Attempts to transition out of a
/// terminal state return [`crate::errors::ContractError::InvalidStatus`].
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimStatus {
    /// The claim has been submitted and is awaiting admin review.
    Pending,
    /// The claim was approved by an admin. **Terminal state.**
    Approved,
    /// The claim was rejected by an admin. **Terminal state.**
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

    /// Returns `true` if this status is a terminal state (no further transitions allowed).
    ///
    /// Terminal states are `Approved` and `Rejected`.  Once a claim reaches
    /// either of these states it cannot be transitioned further.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Approved | Self::Rejected)
    }

    /// Returns `true` if transitioning `self → next` is a valid state-machine step.
    ///
    /// Only `Pending → Approved` and `Pending → Rejected` are valid.
    /// All other combinations (including self-transitions) return `false`.
    pub fn can_transition_to(self, next: ClaimStatus) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Approved) | (Self::Pending, Self::Rejected)
        )
    }
}

/// A pending or resolved ownership-claim request.
///
/// The `status` field follows the [`ClaimStatus`] state machine:
/// `Pending` (initial) → `Approved` or `Rejected` (terminal).
/// See [`ClaimStatus`] for the full state diagram and transition rules.
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

/// The lifecycle state of a fee payment for a single operation.
///
/// # State Machine
///
/// ```text
/// [Unpaid]
///     │  pay_fee() / pay_registration_fee()
///     ▼
/// [Pending]   ← FeePaidForProject flag set, FeePaymentRecord stored
///     │  request_verification() / register_project()
///     │  (consume_fee_payment / consume_registration_fee_payment)
///     ├──────────────────────────────────────────────┐
///     ▼                                              ▼
/// [Consumed]                                   [Cancelled]
///     │  (flag cleared, record retained)            │  cancel_fee_payment()
///     │  reject_verification()                      │  (flag cleared, refund transferred)
///     │  record_verification_refund()               ▼
///     ▼                                         [terminal]
/// [RefundPending]  ← FeeRefundRecord { claimed_at: None }
///     │  claim_fee_refund()
///     ▼
/// [Refunded]       ← FeeRefundRecord { claimed_at: Some(ts) }
/// ```
///
/// # Transition Rules
///
/// | From          | To            | Trigger                             | Guard                          |
/// |---------------|---------------|-------------------------------------|-------------------------------|
/// | Unpaid        | Pending       | `pay_fee` / `pay_registration_fee`  | Token transfer succeeds        |
/// | Pending       | Consumed      | `consume_fee_payment`               | Paid flag set, not expired     |
/// | Pending       | Cancelled     | `cancel_fee_payment`                | Payer or admin; not Pending/Verified |
/// | Consumed      | RefundPending | `record_verification_refund`        | Verification rejected; amount > 0 |
/// | RefundPending | Refunded      | `claim_fee_refund`                  | Payer or admin; not already claimed |
///
/// Any transition not listed above is invalid and returns `InvalidStatus` or
/// `RefundAlreadyClaimed`.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeePaymentStatus {
    /// No fee has been paid yet for this operation.
    Unpaid,
    /// Fee has been paid and is waiting to be consumed by the operation.
    Pending,
    /// Fee was consumed when the operation was submitted for review.
    Consumed,
    /// Fee payment was cancelled and funds were returned to the payer.
    Cancelled,
    /// Verification was rejected; a refund is recorded but not yet claimed.
    RefundPending,
    /// Refund has been paid out to the original payer.
    Refunded,
}

impl FeePaymentStatus {
    /// Validate that a state transition is permitted.
    ///
    /// Returns `Ok(())` for all valid transitions; `Err(InvalidStatus)` for
    /// any invalid transition.  This is the single source of truth for which
    /// transitions are legal in the fee state machine.
    pub fn validate_transition(
        from: FeePaymentStatus,
        to: FeePaymentStatus,
    ) -> Result<(), crate::errors::ContractError> {
        let valid = matches!(
            (from, to),
            (Self::Unpaid, Self::Pending)
                | (Self::Pending, Self::Consumed)
                | (Self::Pending, Self::Cancelled)
                | (Self::Consumed, Self::RefundPending)
                | (Self::RefundPending, Self::Refunded)
        );
        if valid {
            Ok(())
        } else {
            Err(crate::errors::ContractError::InvalidStatus)
        }
    }
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
    /// Admin updated the configurable maximum reviews per project.
    MaxReviewsPerProjectSet,
    /// Admin approved an ownership claim request.
    ClaimRequestApproved,
    /// Admin rejected an ownership claim request.
    ClaimRequestRejected,
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
    /// Maximum number of projects that can be featured simultaneously
    /// (`MAX_FEATURED_PROJECTS`). When this limit is reached, the oldest
    /// featured project is evicted (FIFO) to make room for the new one.
    pub max_featured_projects: u32,
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

// ── Batch TTL extension result (#666) ─────────────────────────────────────────

/// Result returned by batch TTL extension calls.
///
/// The batch is **fail-fast**: the operation stops at the first hard error
/// (e.g. storage failure). Missing projects/reviews are *not* hard errors —
/// they are recorded in `skipped_ids` and processing continues (continue
/// semantics). Only `refreshed` + `skipped` + any partial completion is
/// surfaced here so callers can detect partial-failure states.
///
/// ## Partial-failure states
///
/// | `refreshed` | `skipped_ids.len()` | Meaning |
/// |-------------|---------------------|---------|
/// | N | 0 | Full success, all N items extended |
/// | N | M | Partial: N extended, M not found/skipped |
/// | 0 | M | All items were missing |
///
/// The batch makes a best-effort pass and is **not** transactional: if the
/// call panics mid-way (contract budget exhausted, etc.) some TTLs may have
/// been extended already. Callers that require all-or-nothing semantics should
/// validate all IDs before calling, or check `refreshed == ids.len()`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchTtlResult {
    /// Number of items whose TTL was successfully extended.
    pub refreshed: u32,
    /// IDs that were skipped because the item does not exist in storage.
    /// For project batches these are project IDs; for review batches these
    /// are encoded as `project_id * 1_000_000_007 ^ reviewer_index` — use
    /// `skipped_project_ids` / `skipped_reviewer_indices` instead for reviews.
    pub skipped_ids: Vec<u64>,
}

// ── Recommendation & Recommendation Feedback ──────────────────────────────────

/// Identifies why / how a project was recommended. The recommendation engine
/// (off-chain or future on-chain) sets this when it creates a `Recommendation`
/// so analytics can compare algorithm effectiveness side-by-side.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecommendationAlgorithm {
    /// Simple popularity / most-reviewed ranking.
    Popular,
    /// Highest weighted-rating (see `RatingCalculator`).
    TopRated,
    /// Same category / same tags as a reference project.
    Similar,
    /// Recently registered / trending.
    Trending,
    /// Featured + manually curated admin recommendation.
    Featured,
    /// Personalised for a user (follow graph, bookmarks, endorsements, …).
    Personalised,
    /// Catch-all for any future / custom algorithm.
    Custom,
}

/// What kind of interaction was recorded when tracking a recommendation's
/// click-through and engagement.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecommendationEngagementKind {
    /// Recommendation was rendered and shown to a user (impression). Used
    /// as the denominator for click-through rate.
    Impression,
    /// User clicked / tapped the recommendation card to view the project.
    Click,
    /// User followed the project after arriving via the recommendation.
    Follow,
    /// User bookmarked the project after arriving via the recommendation.
    Bookmark,
    /// User endorsed the project after arriving via the recommendation.
    Endorse,
    /// User submitted a review for the project after arriving via the recommendation.
    Review,
}

/// A single recommendation. Each recommendation points at a single *target*
/// project (`target_project_id`) and is labelled with the algorithm that
/// produced it. Optional `reference_project_id` + `audience` fields make it
/// possible to group recommendations by the context in which they were shown
/// (e.g. "similar to project X" vs "for-you feed for user Y").
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recommendation {
    pub id: u64,
    /// Project being recommended (what the user will click into).
    pub target_project_id: u64,
    /// Algorithm used to produce the recommendation.
    pub algorithm: RecommendationAlgorithm,
    /// Optional reference project used as the seed for "similar" recs.
    pub reference_project_id: Option<u64>,
    /// Optional audience user the recommendation was personalised for.
    pub audience: Option<Address>,
    /// Algorithm-provided score / confidence (unsigned integer, unscaled;
    /// higher = stronger signal). `None` for unranked recommendations.
    pub score: Option<u64>,
    /// Free-form short label ("trending now", "you may like", …).
    pub label: Option<String>,
    /// Ledger timestamp when this recommendation was created.
    pub created_at: u64,
}

/// Per-user, per-recommendation thumbs-up / thumbs-down feedback. Stored
/// explicitly (rather than aggregated into a counter) so recommendation
/// engines can inspect *who* liked or disliked a recommendation, which
/// enables collaborative filtering and fraud / Sybil detection off-chain.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecommendationFeedback {
    pub recommendation_id: u64,
    pub user: Address,
    /// `true` = thumbs up / helpful; `false` = thumbs down / not helpful.
    pub helpful: bool,
    /// Ledger timestamp when feedback was submitted.
    pub created_at: u64,
}

/// Aggregated, read-only analytics snapshot for a single recommendation.
/// Returned by `get_recommendation_analytics` so indexers and UIs can
/// present effectiveness numbers without doing N storage reads on the client.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecommendationAnalytics {
    pub recommendation_id: u64,
    /// Number of times the recommendation was marked as rendered (impressions).
    pub impressions: u64,
    /// Number of clicks on the recommendation card.
    pub clicks: u64,
    /// Click-through rate scaled by `1_000_000` (ppm).
    /// `clicks / impressions * 1_000_000`; 0 if impressions == 0.
    pub click_through_rate_ppm: u32,
    /// Thumbs-up count (helpful == true).
    pub helpful_count: u64,
    /// Thumbs-down count (helpful == false).
    pub not_helpful_count: u64,
    /// Helpful ratio scaled by `1_000_000` (ppm).
    /// `helpful_count / total_feedback * 1_000_000`; 0 if no feedback exists.
    pub helpful_ratio_ppm: u32,
    /// Count of Follow engagements triggered from this recommendation.
    pub follow_engagements: u64,
    /// Count of Bookmark engagements triggered from this recommendation.
    pub bookmark_engagements: u64,
    /// Count of Endorse engagements triggered from this recommendation.
    pub endorse_engagements: u64,
    /// Count of Review engagements triggered from this recommendation.
    pub review_engagements: u64,
    /// Composite effectiveness score (0–10,000 basis points) combining
    /// CTR, helpful ratio and downstream engagement signals. Used by
    /// `list_recommendations_sorted_by_effectiveness` and by recommendation
    /// engines to "improve based on feedback".
    pub effectiveness_score_bps: u32,
}

// ── Community Collections (Issue #821) ──────────────────────────────────────

/// Who owns or stewards a community collection.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommunityCollectionRole {
    /// Original creator of the collection. Receives creator-level revenue
    /// share (if any) and cannot be removed from the curator set.
    Creator,
    /// Ordinary curator: can add/remove projects, update metadata, but does
    /// not collect creator-level revenue.
    Curator,
}

/// A single up-or-down community vote to include a project in a curated
/// collection (AC2 — the curation mechanism). `approve = true` is a "yay" vote
/// to include (or keep), `approve = false` is a "nay" vote to exclude (or
/// drop). Append-only per voter per collection per project; repeat submissions
/// error with `CommunityColVoteAlreadyCast`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommunityCollectionVote {
    pub collection_id: u64,
    pub project_id: u64,
    pub voter: Address,
    pub approve: bool,
    pub created_at: u64,
}

/// Describes the inclusion state of a single project in a community collection
/// after aggregating all votes and curator actions. Used to produce a
/// definitive "is this project in the collection?" answer for UI display.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommunityColInclusionStatus {
    /// Project is in the collection (either directly added by a curator or
    /// crossed the approval-vote threshold).
    Included,
    /// Project is not in the collection (either never added, removed by a
    /// curator, or crossed the disapproval-vote threshold).
    Excluded,
    /// Project was up for inclusion and a vote is running, but no threshold
    /// has been reached yet. Only used as a return value for projects that
    /// have at least one vote cast and are not yet explicitly Added/Removed.
    Pending,
}

/// The identity of a pre-defined collection template (AC4 — templates for
/// common collections). A template is a collection record with `is_template =
/// true`. Callers can clone a template into a new community collection via
/// `create_community_collection_from_template`, which copies the metadata
/// description/project-set skeleton from the template into a new collection
/// owned by the caller.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommunityCollectionTemplateId {
    /// "Stellar DeFi Darlings" — well-known liquidity pools, DEXs, lending
    /// and borrowing products. Seeded project list is empty on deploy;
    /// populated by admins (avoids tying Soroban contract deploy to any
    /// particular set of IDs).
    Defi,
    /// "NFT / Marketplaces" — marketplaces, trading venues, minting tools.
    Nft,
    /// "DAO / Governance Tools" — DAO frameworks, voting, treasury,
    /// multisig.
    Dao,
    /// "Gaming / Metaverse" — on-chain game worlds, land, in-game assets.
    Gaming,
    /// "Infrastructure / Tooling" — oracles, RPC, bridges, block explorers,
    /// indexing, SDKs.
    Infra,
    /// "Stablecoins / Payments" — fiat-backed / algorithmic stablecoins and
    /// payment-focused contracts.
    Stablecoins,
    /// "Sustainability / Public Goods" — retroactive public-goods funding,
    /// carbon, R&D grants, open source stewards.
    PublicGoods,
    /// "Audited & Verified" — project set curated from the registry's
    /// verified set; a starting point for users who want to trust but verify.
    Verified,
}

/// Community collection (AC1-4). Unlike `Collection` (admin-only), this is
/// user-created, has curator voting, can be featured by admin, participates
/// in revenue sharing, and supports template cloning.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommunityCollection {
    pub id: u64,
    /// Creator / initial curator. Always authenticated when creating the
    /// collection. Receives the creator share of any revenue (AC3).
    pub creator: Address,
    /// Curator set (addresses who can add/remove projects directly). The
    /// creator is NOT implicitly duplicated here; both the creator and any
    /// address in this list can perform curator actions. Kept separate so
    /// the creator role can receive distinct revenue shares.
    pub curators: Vec<Address>,
    /// Required approval votes for a community-proposed project addition to
    /// become `Included` without a direct curator add. 0 disables voting
    /// gates entirely.
    pub approval_threshold: u32,
    /// Required disapproval votes for a community-proposed removal to become
    /// `Excluded` without a direct curator remove. 0 disables removal voting.
    pub disapproval_threshold: u32,
    pub name: String,
    pub description: String,
    /// Optional free-form tag string for indexers / UI facets (comma-
    /// separated, unstructured on-chain). E.g. "defi,nft,verified".
    pub tags: Option<String>,
    /// When true this collection is a template (AC4). Templates cannot hold
    /// votes or revenue; they exist to be cloned via
    /// `create_community_collection_from_template`.
    pub is_template: bool,
    /// Which pre-defined template id (if any) this collection was cloned
    /// from. `None` for collections created from scratch.
    pub template_source: Option<CommunityCollectionTemplateId>,
    /// Admin-only flag (AC1). When true the collection is surfaced in the
    /// featured-community-collections list. Set by admin via
    /// `feature_community_collection`.
    pub is_featured: bool,
    /// Basis points of any attributable revenue distributed to the creator
    /// (AC3). The remaining share is distributed evenly across the curator
    /// set. Creator share + (curator share per curator) ≤ 10_000.
    pub creator_revenue_share_bps: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Snapshot of the accrued revenue attribution (AC3) for a single community
/// collection. Recorded as cumulative totals so off-chain indexers can
/// distribute payouts at any cadence without needing on-chain transfer logic.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommunityColRevenueSnapshot {
    pub collection_id: u64,
    /// Cumulative attributed tokens (1e7 scaled) to the creator address.
    pub creator_cumulative_attributed: u128,
    /// Cumulative attributed tokens (1e7 scaled) to the curator set, split
    /// evenly. Per-curator = `curators_cumulative_attributed / N_curators`.
    pub curators_cumulative_attributed: u128,
    /// Cumulative total attributed tokens (for reconciliation / sanity).
    pub total_cumulative_attributed: u128,
    /// Ledger timestamp when this snapshot was emitted.
    pub as_of_timestamp: u64,
}

// ── Social Analytics (Issue #822) ──────────────────────────────────────────

/// One day of social-signal snapshots for a project. Keyed on the calendar day
/// (Unix timestamp / 86400). Stored every time a caller invokes
/// `record_project_social_daily_checkpoint`; the latest value per day wins
/// (callers are expected to checkpoint roughly once every 24 hours).
///
/// Every counter is stored as a cumulative **snapshot count**, NOT a daily
/// delta. Deltas between two days are derived by subtracting the earlier
/// snapshot from the later snapshot — this makes any 2-window comparison
/// (`last_7_days`, `last_30_days`, arbitrary ranges) trivial to compute
/// without needing to iterate every day in between.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSocialDailyCheckpoint {
    pub project_id: u64,
    /// Day index = ledger timestamp / 86_400 when this checkpoint was recorded.
    pub day_index: u32,
    /// Unix timestamp when this checkpoint was persisted (may be later than
    /// the day_start if the caller checkpoints early in the day).
    pub recorded_at: u64,
    pub follower_count: u32,
    pub endorsement_count: u32,
    pub bookmark_count: u32,
    pub review_count: u32,
    /// Average review rating scaled in basis points (0–50_000 for 0–5 stars).
    pub average_rating_bps: u32,
    /// Sum of follower + endorsement + bookmark + review counts on the day
    /// of the checkpoint. Pre-summed so downstream aggregation can avoid
    /// re-adding per-window.
    pub total_engagement_units: u64,
}

/// Engagement rate metric for a project over an arbitrary time window (AC2).
/// Engagement rate is computed as `(follower_gain + endorsement_gain +
/// bookmark_gain + review_gain) / (follower_count_start + 1)` scaled to
/// **parts per million** (ppm) so ratios remain integer-only for `no_std`
/// environments. The "+1" stabilises the denominator on brand-new projects
/// so we do not divide by zero.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectEngagementMetric {
    pub project_id: u64,
    /// Start day index of the window (inclusive).
    pub window_start_day: u32,
    /// End day index of the window (inclusive).
    pub window_end_day: u32,
    /// Net follower gain inside the window (snapshot end − snapshot start).
    pub follower_gain: i64,
    pub endorsement_gain: i64,
    pub bookmark_gain: i64,
    pub review_gain: i64,
    /// Sum of all four gains (absolute-value clamped to ≥0 so ppm ratio is
    /// never negative).
    pub net_engagement_gain: u64,
    /// Follower count at the beginning of the window (or 0 if no checkpoint
    /// existed pre-window; we use `max(start_follower_count, 1)` as
    /// denominator).
    pub start_follower_count: u32,
    /// Engagement rate expressed in parts-per-million (ppm)
    /// `= net_engagement_gain * 1_000_000 / max(start_follower_count, 1)`.
    pub engagement_rate_ppm: u64,
    /// Average rating change (bps) over the window (end_avg − start_avg).
    /// May be negative if average rating dropped.
    pub rating_delta_bps: i64,
}

/// A single peer-project row returned by the comparison endpoint (AC3).
/// Includes only the numbers needed for UI comparison widgets (percentile
/// ranking is derived client-side from the returned sorted list).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSocialPeerRow {
    pub project_id: u64,
    /// Category-matched peer project name for convenience (saves UI one
    /// `get_project` round-trip per row).
    pub project_name: String,
    /// Engagement rate ppm over the same comparison window as the queried
    /// project.
    pub engagement_rate_ppm: u64,
    /// Net engagement gain in the comparison window (units of
    /// follower + endorse + bookmark + review).
    pub net_engagement_gain: u64,
    /// Snapshot follower count at the end of the comparison window (latest
    /// available checkpoint; falls back to live count if none).
    pub latest_follower_count: u32,
    /// Average rating in bps at end of window (or current live stat).
    pub average_rating_bps: u32,
}

/// AC4: the fully-loaded "export analytics report" payload. Includes every
/// metric a downstream report or indexer would need so consumers don't have
/// to re-assemble 6–7 endpoints.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSocialAnalyticsExport {
    pub project_id: u64,
    /// Project-category string (for reproducibility of peer comparison).
    pub category: String,
    /// Number of daily checkpoints stored for this project.
    pub checkpoint_count: u32,
    /// Oldest / newest checkpoint day indices — lets the caller know the
    /// report's data horizon.
    pub oldest_checkpoint_day: Option<u32>,
    pub newest_checkpoint_day: Option<u32>,
    /// 7-day engagement rate (AC2) computed on the fly.
    pub last_7_days: ProjectEngagementMetric,
    /// 30-day engagement rate (AC2) computed on the fly.
    pub last_30_days: ProjectEngagementMetric,
    /// Growth numbers of pure follower + endorsement + review + bookmark
    /// counts in the last 30 days (AC1). Matches the `net_engagement_gain`
    /// of the `last_30_days` metric but is replicated here at top level so
    /// CSV exporters can extract it in a flat column.
    pub growth_last_30_days_total_engagement: u64,
    pub growth_last_30_days_followers: i64,
    pub growth_last_30_days_endorsements: i64,
    pub growth_last_30_days_bookmarks: i64,
    pub growth_last_30_days_reviews: i64,
    /// Peer comparison rows (AC3) over last-30-day window, sorted by
    /// `engagement_rate_ppm` descending (peer with highest rate at index 0,
    /// target project always included so percentile is caller-computable).
    pub peer_comparison: Vec<ProjectSocialPeerRow>,
    /// 0-based position of the target project inside `peer_comparison` after
    /// sorting (so the caller can compute percentile = pos / len).
    pub self_index_in_peer_ranking: u32,
    /// Unix ledger timestamp when this report was generated.
    pub generated_at: u64,
}
