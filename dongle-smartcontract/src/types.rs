use soroban_sdk::{contracttype, Address, Map, String, Vec};

/// Parameters supplied to `register_project`. All required fields must be
/// non-empty; optional fields default to `None` when omitted.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ProjectRegistrationParams {
    /// Address that will own the project and may update it.
    pub owner: Address,
    /// Human-readable project name (max `MAX_NAME_LEN` bytes, non-empty).
    pub name: String,
    /// URL-safe slug used as a stable identifier in links and queries
    /// (max `MAX_SLUG_LEN` bytes, `[a-z0-9-]` only, non-empty).
    pub slug: String,
    /// Short description of the project (max `MAX_DESCRIPTION_LEN` bytes).
    pub description: String,
    /// Project category label (max `MAX_CATEGORY_LEN` bytes, non-empty).
    pub category: String,
    /// Project website URL (max `MAX_WEBSITE_LEN` bytes, must start with
    /// `https://` when present).
    pub website: Option<String>,
    /// SPDX license identifier or free-text license note
    /// (max `MAX_LICENSE_LEN` bytes).
    pub license: Option<String>,
    /// IPFS CID of the project's logo image (max `MAX_CID_LEN` bytes).
    pub logo_cid: Option<String>,
    /// IPFS CID of the off-chain metadata document (max `MAX_CID_LEN` bytes).
    /// Must conform to `docs/project-metadata.schema.json`.
    pub metadata_cid: Option<String>,
    /// Up to `MAX_TAGS_PER_PROJECT` short tag strings
    /// (each max `MAX_TAG_LENGTH` bytes, `[A-Za-z0-9_-]` only).
    pub tags: Option<Vec<String>>,
    /// Map of platform name → URL for social/community links
    /// (max `MAX_SOCIAL_LINKS` entries; key max `MAX_SOCIAL_LINK_PLATFORM_LEN`,
    /// value max `MAX_SOCIAL_LINK_URL_LEN`).
    pub social_links: Option<Map<String, String>>,
    /// Unix timestamp (seconds) when the project publicly launched.
    /// Optional; purely informational for frontends.
    pub launch_timestamp: Option<u64>,
    /// URL of the project's bug-bounty or security-disclosure programme.
    pub bounty_url: Option<String>,
    /// URL of the project's source-code repository.
    pub repository_url: Option<String>,
}

/// Parameters supplied to `update_project`. Each field is an `Option`
/// containing the new value; `None` means "leave unchanged". Fields that are
/// themselves nullable on the `Project` struct are wrapped in `Option<Option<T>>`
/// so callers can explicitly clear them (`Some(None)`) versus leaving them
/// alone (`None`).
///
/// **Note:** `lifecycle_status` is intentionally absent here — it has its own
/// entry point (`set_project_lifecycle_status`) which emits a dedicated event.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ProjectUpdateParams {
    /// ID of the project to update.
    pub project_id: u64,
    /// Address calling the update; must be the project owner.
    pub caller: Address,
    /// New project name, or `None` to leave unchanged.
    pub name: Option<String>,
    /// New URL-safe slug, or `None` to leave unchanged.
    /// Immutable once the project is `Verified`.
    pub slug: Option<String>,
    /// New description, or `None` to leave unchanged.
    pub description: Option<String>,
    /// New category, or `None` to leave unchanged.
    pub category: Option<String>,
    /// `Some(Some(url))` to set, `Some(None)` to clear, `None` to leave unchanged.
    pub website: Option<Option<String>>,
    /// `Some(Some(id))` to set, `Some(None)` to clear, `None` to leave unchanged.
    pub license: Option<Option<String>>,
    /// `Some(Some(cid))` to set, `Some(None)` to clear, `None` to leave unchanged.
    pub logo_cid: Option<Option<String>>,
    /// `Some(Some(cid))` to set, `Some(None)` to clear, `None` to leave unchanged.
    /// Changing this field invalidates an active verification (major-metadata rule).
    pub metadata_cid: Option<Option<String>>,
    /// `Some(Some(tags))` to replace the tag list, `Some(None)` to clear,
    /// `None` to leave unchanged.
    pub tags: Option<Option<Vec<String>>>,
    /// `Some(Some(map))` to replace social links, `Some(None)` to clear,
    /// `None` to leave unchanged.
    pub social_links: Option<Option<Map<String, String>>>,
    /// `Some(Some(ts))` to set launch timestamp, `Some(None)` to clear,
    /// `None` to leave unchanged.
    pub launch_timestamp: Option<Option<u64>>,
    /// `Some(Some(url))` to set bounty URL, `Some(None)` to clear,
    /// `None` to leave unchanged.
    pub bounty_url: Option<Option<String>>,
    /// `Some(Some(url))` to set repository URL, `Some(None)` to clear,
    /// `None` to leave unchanged.
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

/// Aggregated rating statistics for a single project.
///
/// Stored separately from `Project` so it can be updated on every review
/// write without re-serialising the full project entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectStats {
    /// Sum of all individual ratings (1–5 each). Divide by `review_count` to
    /// compute the raw average, or use `average_rating` for the pre-computed
    /// Bayesian-weighted value.
    pub rating_sum: u64,
    /// Total number of non-deleted reviews submitted for this project.
    /// Deleted reviews are excluded and their ratings are subtracted from
    /// `rating_sum` at deletion time.
    pub review_count: u32,
    /// Bayesian-weighted average rating scaled by 100 (e.g. 350 = 3.50 stars).
    /// Computed as:
    /// `(prior_count × prior_mean + rating_sum) / (prior_count + review_count)`
    /// See `WEIGHTED_RATING_PRIOR_COUNT` and `WEIGHTED_RATING_PRIOR_MEAN` in
    /// `constants.rs` for the prior parameters.
    pub average_rating: u32,
}

/// A single URL attached to a review as supporting evidence.
///
/// The `is_dead` flag is set by admins via `mark_evidence_link_dead` when a
/// link is found to be broken or invalid. It is stored inline so the link
/// record is preserved for audit purposes even after it is marked dead.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceLink {
    /// The URL string (http:// or https://).
    pub url: String,
    /// Admin-settable dead-link flag. `false` by default.
    /// Set to `true` via `mark_evidence_link_dead` when a link is broken or invalid.
    pub is_dead: bool,
}

/// A single on-chain review submitted for a project.
///
/// Reviews are keyed by `(project_id, reviewer)` — one review per reviewer
/// per project. Use `update_review` to change the rating or content.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    /// ID of the project being reviewed.
    pub project_id: u64,
    /// Address of the reviewer. Unique per project.
    pub reviewer: Address,
    /// Rating in the range `[RATING_MIN, RATING_MAX]` (currently 1–5 inclusive).
    pub rating: u32,
    /// Canonical content CID - replaces the redundant ipfs_cid/comment_cid pair.
    /// Points to an off-chain JSON document conforming to `docs/review-cid.schema.json`.
    /// `None` when the review was submitted without off-chain content.
    pub content_cid: Option<String>,
    /// Optional reply written by the project owner in response to this review.
    /// Set via `set_owner_response`; only the current project owner may write it.
    pub owner_response: Option<String>,

    /// Unix timestamp (seconds) when the review was first submitted.
    pub created_at: u64,

    /// Unix timestamp (seconds) of the most recent modification to this review.
    pub updated_at: u64,

    /// Unix timestamp (seconds) of the most recent reviewer update.
    /// Zero means the review has not been updated since submission.
    pub last_updated_at: u64,

    /// Whether the review is hidden by moderation.
    /// Set to `true` by admins via `hide_review`; cleared by `restore_review`.
    /// Hidden reviews are excluded from public listing endpoints but the record
    /// is retained on-chain for auditability.
    pub hidden: bool,

    /// Number of times this review has been reported via `report_review`.
    /// Admins may inspect high-count reviews and choose to hide them.
    /// Does not automatically trigger hiding — that requires an explicit admin
    /// action.
    pub report_count: u32,

    /// Optional list of attached evidence links (max MAX_EVIDENCE_LINKS_PER_REVIEW).
    pub evidence_links: Vec<EvidenceLink>,
}

/// Identifies the lifecycle event that produced a `ReviewEventData` emission.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewAction {
    /// First submission of the review via `submit_review` / `add_review`.
    Submitted,
    /// Reviewer updated rating or content via `update_review`.
    Updated,
    /// Internal edit step that also stored a `ReviewRevision` snapshot.
    Revised,
    /// Review deleted by the reviewer or an admin via `delete_review`.
    Deleted,
}

/// Payload carried by review lifecycle events (submitted, updated, deleted).
/// Mirrors the most-read fields of `Review` so indexers do not need a
/// separate storage read after observing an event.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewEventData {
    /// ID of the reviewed project.
    pub project_id: u64,
    /// Address that owns the review.
    pub reviewer: Address,
    /// Lifecycle event that triggered this emission.
    pub action: ReviewAction,
    /// Ledger timestamp (seconds) at the time of the event.
    pub timestamp: u64,
    /// Canonical content CID - consolidates the review content.
    pub content_cid: Option<String>,
    /// Project owner's reply at the time of the event (may be stale for
    /// `Updated` / `Revised` events if the response was set independently).
    pub owner_response: Option<String>,
    /// Original submission timestamp; unchanged across edits.
    pub created_at: u64,
    /// Timestamp of the most recent modification at the time of the event.
    pub updated_at: u64,
    /// Snapshot of evidence links at the time this event was emitted.
    pub evidence_links: Vec<EvidenceLink>,
}

/// Snapshot of a review before an edit. Stored in ascending revision_index order (0 = first edit).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewRevision {
    /// Zero-based position of this revision in the edit history for the review.
    /// Increments by 1 on each `update_review` call that actually changes
    /// content. Oldest revisions are dropped when `MAX_REVIEW_REVISIONS` is
    /// exceeded (the index value of retained revisions is not renumbered).
    pub revision_index: u32,
    /// Rating at the time this snapshot was taken (before the edit that
    /// produced this revision).
    pub rating: u32,
    /// Content CID at the time of the snapshot (`None` if there was no
    /// off-chain content before the edit).
    pub content_cid: Option<String>,
    /// Unix timestamp (seconds) when this revision was recorded.
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
    /// ID of the reviewed project.
    pub project_id: u64,
    /// Address that owns the review.
    pub reviewer: Address,
    /// Zero-based index of the revision being recorded (matches the
    /// corresponding `ReviewRevision::revision_index`).
    pub revision_index: u32,
    /// Rating value before this edit.
    pub previous_rating: u32,
    /// Content CID before this edit (`None` if there was none).
    pub previous_content_cid: Option<String>,
    /// Rating value after this edit.
    pub new_rating: u32,
    /// Content CID after this edit (`None` if cleared).
    pub new_content_cid: Option<String>,
    /// Ledger timestamp (seconds) of the edit.
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
    /// Unique monotonically-increasing claim identifier.
    pub id: u64,
    /// ID of the project being claimed.
    pub project_id: u64,
    /// Address that submitted the claim.
    pub claimant: Address,
    /// IPFS CID of the proof document supplied by the claimant.
    pub proof_cid: String,
    /// Current status of the claim (see [`ClaimStatus`]).
    pub status: ClaimStatus,
    /// Unix timestamp (seconds) when the claim was submitted.
    pub created_at: u64,
}

/// A pending or resolved contract-address claim for a project.
///
/// Lets a project owner prove on-chain that a given Stellar contract address
/// belongs to their project. The workflow mirrors ownership claims but is
/// specific to contract-address attestation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaimRequest {
    /// ID of the project the contract address is being claimed for.
    pub project_id: u64,
    /// The Stellar contract address (56-char Strkey starting with `C`)
    /// being claimed.
    pub contract_address: String,
    /// Address that submitted the claim.
    pub claimant: Address,
    /// IPFS CID of the proof document supplied by the claimant.
    pub proof_cid: String,
    /// Current status of the claim (see [`ClaimStatus`]).
    pub status: ClaimStatus,
    /// Unix timestamp (seconds) when the claim was submitted.
    pub created_at: u64,
    /// Unix timestamp (seconds) after which this pending claim is considered expired.
    /// A value of 0 means no expiry (legacy). New claims always set this to
    /// `created_at + CLAIM_EXPIRY_SECONDS`.
    pub expires_at: u64,
}

/// The primary on-chain record for a registered project.
///
/// Written by `register_project`; mutated by `update_project`,
/// `archive_project`, `reactivate_project`, `initiate_transfer`,
/// `accept_transfer`, and several verification/verification-renewal paths.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// Unique monotonically-increasing project identifier assigned at
    /// registration time.
    pub id: u64,
    /// Address that currently owns the project. Updated by
    /// `accept_transfer` when an ownership transfer completes.
    pub owner: Address,
    /// Human-readable project name (max `MAX_NAME_LEN` bytes).
    /// Changing this field while the project is `Verified` invalidates the
    /// current verification (major-metadata rule).
    pub name: String,
    /// URL-safe slug (`[a-z0-9-]`). Used as a stable human-readable key in
    /// external links. Immutable once the project is `Verified`.
    pub slug: String,
    /// Short description of the project.
    pub description: String,
    /// Category label (e.g. "DeFi", "NFT", "Infrastructure").
    pub category: String,
    /// Project website URL. Changing this field while `Verified` invalidates
    /// the current verification (major-metadata rule).
    pub website: Option<String>,
    /// SPDX license identifier or free-text note (e.g. "Apache-2.0").
    pub license: Option<String>,
    /// IPFS CID of the project's logo image.
    pub logo_cid: Option<String>,
    /// IPFS CID of the off-chain metadata document. Changing this field while
    /// `Verified` invalidates the current verification (major-metadata rule).
    /// Must conform to `docs/project-metadata.schema.json`.
    pub metadata_cid: Option<String>,
    /// Current verification status of the project. Transitions are validated
    /// by `VerificationStateMachine` — see `verification_registry/state_machine.rs`.
    pub verification_status: VerificationStatus,
    /// ID of the most recent active `VerificationRecord` for this project.
    /// `None` until the first verification request is submitted. Updated each
    /// time a new request is created (the old record is retained for history).
    pub current_verification_id: Option<u64>,
    /// Whether the project has been archived by its owner.
    /// Archived projects cannot be updated, receive reviews, or request
    /// verification until reactivated via `reactivate_project`.
    pub archived: bool,
    /// Whether the project is open for ownership claims.
    /// Set by the current owner via `set_project_claimable`. When `true`,
    /// any address may submit a claim request; when `false` claims are blocked.
    pub claimable: bool,
    /// Lifecycle maturity signal set by the owner via
    /// `set_project_lifecycle_status`. Used by frontends to surface project
    /// stability (Active, Beta, Paused, Deprecated, Sunset). Does not affect
    /// any on-chain permissions.
    pub lifecycle_status: ProjectLifecycleStatus,
    /// Unix timestamp (seconds) when the project was first registered.
    pub created_at: u64,
    /// Unix timestamp (seconds) of the most recent update to any project field.
    pub updated_at: u64,
    /// Up to `MAX_TAGS_PER_PROJECT` short tag strings for discovery.
    pub tags: Option<Vec<String>>,
    /// Map of platform name → URL for social/community links.
    pub social_links: Option<Map<String, String>>,
    /// Optional Unix timestamp (seconds) of the project's public launch.
    /// Informational only.
    pub launch_timestamp: Option<u64>,
    /// Optional list of maintainer addresses. Maintainers have no on-chain
    /// privilege beyond being listed here; they cannot update the project or
    /// request verification on the owner's behalf.
    pub maintainers: Option<Vec<Address>>,
    /// URL of the project's bug-bounty or security-disclosure programme.
    pub bounty_url: Option<String>,
    /// URL of the project's source-code repository.
    pub repository_url: Option<String>,
    /// Published security contact (e-mail, URL, or `security.txt` reference).
    /// Settable by the owner via `set_security_contact`.
    pub security_contact: Option<String>,
    /// IPFS CID of the proof document for the security contact attestation.
    /// Set alongside `security_contact` when the owner calls
    /// `set_security_contact` with a non-empty proof CID.
    /// Cleared when `security_contact` is removed.
    pub security_contact_proof_cid: Option<String>,
    /// Whether the security contact has been verified by an admin via
    /// `verify_security_contact`. Automatically cleared when
    /// `security_contact` is updated or removed.
    pub security_contact_verified: bool,
}

/// Read-only view of a project's security contact, returned by
/// `get_security_contact_status`. Avoids fetching the full `Project`
/// struct when only the security-contact fields are needed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityContactStatus {
    /// The published security contact string (e-mail, URL, etc.), if any.
    pub contact: Option<String>,
    /// IPFS CID of the supporting proof document, if provided.
    pub proof_cid: Option<String>,
    /// Whether the contact has been verified by an admin.
    pub verified: bool,
}

/// A moderation report submitted against a project.
///
/// Reports are stored as a `Vec<ProjectReport>` under
/// `ExtensionKey::ProjectReports(project_id)`. The count is cached
/// separately for cheap access without deserialising the full list.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReport {
    /// ID of the reported project.
    pub project_id: u64,
    /// Address that submitted the report.
    pub reporter: Address,
    /// IPFS CID of the report content / evidence document.
    pub reason_cid: String,
    /// Unix timestamp (seconds) when the report was submitted.
    pub timestamp: u64,
}

/// Verification lifecycle status for a project.
///
/// Transitions are enforced by `VerificationStateMachine`. Only the paths
/// below are valid; any other transition returns `InvalidStatus`.
///
/// ```text
/// Unverified ──request──► Pending ──approve──► Verified ──revoke──► Unverified
///                │                  └──reject──► Rejected
///                ◄──────────── re-request ───────────┘
/// ```
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    /// No verification request has been submitted, or a previous verification
    /// was revoked by an admin.
    Unverified,
    /// A verification request has been submitted and is awaiting admin review.
    Pending,
    /// The project has been verified by an admin.
    Verified,
    /// The most recent verification request was rejected by an admin.
    /// The owner may re-pay the fee and re-submit.
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

/// Complete record of a single verification request, including its outcome.
///
/// One record is created per `request_verification` call. Records are
/// immutable after their final state is set (`decided_at > 0`). The
/// project's `current_verification_id` always points to the most recently
/// created record; older records are retained for history.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRecord {
    /// Unique monotonically-increasing identifier for this request.
    pub request_id: u64,
    /// ID of the project this request belongs to.
    pub project_id: u64,
    /// Address that submitted the verification request (must be the
    /// project owner at the time of submission).
    pub requester: Address,
    /// Current status of this record. Starts as `Pending`; transitions to
    /// `Verified` on approval, `Rejected` on rejection, or `Unverified` on
    /// revocation.
    pub status: VerificationStatus,
    /// IPFS CID of the evidence document provided by the requester.
    /// Must conform to `docs/verification-evidence.schema.json`.
    pub evidence_cid: String,
    /// Unix timestamp (seconds) when the request was submitted.
    pub requested_at: u64,
    /// Unix timestamp (seconds) when an admin approved, rejected, or revoked
    /// this request. Zero while the request is still `Pending`.
    pub decided_at: u64,
    /// Fee amount (in the smallest unit of the configured token) that was
    /// consumed when this request was submitted. Stored here so a refund can
    /// be issued without re-reading the payment record (which is cleared on
    /// consumption).
    pub fee_amount: u128,
    /// Optional admin-supplied reason for a revocation. Non-`None` only when
    /// `status == Unverified` and the transition was triggered by an admin
    /// calling `revoke_verification`.
    pub revoke_reason: Option<String>,
    /// Unix timestamp when verification expires (0 = no expiry).
    /// Set to `requested_at + verification_duration` on approval. After this
    /// timestamp the project's verified status is considered lapsed and it
    /// must renew via `request_renewal`.
    pub expires_at: u64,
    /// Unix timestamp when verification was last renewed.
    /// Updated by `approve_renewal`; zero until the first renewal.
    pub last_renewed_at: u64,
    /// Admin assigned to review this verification request via
    /// `assign_verification`. `None` until explicitly assigned.
    pub assigned_admin: Option<Address>,
}

/// Record of a completed verification renewal.
///
/// Created by `approve_renewal` and stored alongside the canonical
/// `VerificationRecord` to give indexers a full renewal history.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewalRecord {
    /// ID of the project whose verification was renewed.
    pub project_id: u64,
    /// Address that submitted the renewal request (must be the project owner).
    pub requester: Address,
    /// Status of the project's verification at the time of renewal
    /// (always `Verified` for a successful renewal record).
    pub status: VerificationStatus,
    /// IPFS CID of the evidence document submitted with the renewal request.
    pub evidence_cid: String,
    /// Unix timestamp (seconds) when the renewal was approved.
    pub timestamp: u64,
    /// Fee amount (in the smallest unit of the configured token) consumed for
    /// this renewal.
    pub fee_amount: u128,
    /// Unix timestamp when the renewed verification expires.
    pub expires_at: u64,
}

/// Fee configuration for contract operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    /// SAC token address used for fee payments, or `None` when fees are
    /// free (zero-amount). When non-`None`, `verification_fee` and
    /// `registration_fee` must be transferred in this token.
    pub token: Option<Address>,
    /// Fee amount (smallest token unit) charged per `request_verification`
    /// call. Zero disables the verification fee.
    pub verification_fee: u128,
    /// Fee amount (smallest token unit) charged per `register_project` call.
    /// Zero disables the registration fee.
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

/// Record of a completed fee payment. Stored under
/// `ExtensionKey::FeePaymentDetails(project_id)` for verification fees
/// or `ExtensionKey::RegistrationFeePaymentDetails(address)` for
/// registration fees. Retained as an audit trail even after the payment
/// flag is consumed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaymentRecord {
    /// Unix timestamp (seconds) when the payment was made.
    pub paid_at: u64,
    /// Address that executed the payment (must be the project owner or
    /// registrant).
    pub payer: Address,
    /// Amount paid in the smallest unit of `token`.
    pub amount: u128,
    /// Token the fee was paid in. `None` when the fee was configured as
    /// free (zero-amount) and no token transfer occurred.
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
    /// Accumulated across multiple rejections if the owner re-paid and was
    /// rejected again before claiming the outstanding refund.
    pub amount: u128,
    /// Token the fee was paid in. `None` when the fee was configured as free.
    pub token: Option<Address>,
    /// Ledger timestamp at which the refund became claimable.
    pub created_at: u64,
    /// Ledger timestamp of the payout, or `None` while still outstanding.
    pub claimed_at: Option<u64>,
}

/// Immutable snapshot of one fee-configuration change, appended to the history
/// list whenever `set_fee` (or an equivalent timelock/proposal path) succeeds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigHistoryEntry {
    /// Admin address that executed the change.
    pub admin: Address,
    /// Token address before the change, or `None` if fees were free.
    pub old_token: Option<Address>,
    /// Verification fee before the change, or `None` for the initial entry.
    pub old_verification_fee: Option<u128>,
    /// Registration fee before the change, or `None` for the initial entry.
    pub old_registration_fee: Option<u128>,
    /// Treasury address before the change, or `None` for the initial entry.
    pub old_treasury: Option<Address>,
    /// New token address after the change.
    pub token: Option<Address>,
    /// New verification fee after the change.
    pub verification_fee: u128,
    /// New registration fee after the change.
    pub registration_fee: u128,
    /// New treasury address after the change.
    pub treasury: Address,
    /// Unix timestamp (seconds) when the change was applied.
    pub timestamp: u64,
}

// ── Project dependencies ─────────────────────────────────────────────────────

/// External dependency reference can point to an internal project id,
/// an external IPFS CID, an external URL, or a Stellar contract address.
///
/// Exactly one of the four fields should be `Some`; the others should be
/// `None`. Using more than one field in a single `DependencyRef` is
/// permitted by the type but semantically ambiguous.
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

/// A single entry in a project's dependency list.
///
/// Dependencies are stored in a `Vec<ProjectDependency>` under
/// `ExtensionKey::ProjectDependencies(project_id)`. Adding a dependency
/// that creates a cycle or exceeds `MAX_DEPENDENCY_DEPTH` is rejected.
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
    /// ID of the project whose featured status changed.
    pub project_id: u64,
    /// `true` when the project was featured; `false` when unfeatured.
    pub featured: bool,
    /// Admin address that made the change.
    pub admin: Address,
    /// Unix timestamp (seconds) when the change was applied.
    pub timestamp: u64,
}

/// A curated collection of projects, managed by admins.
///
// ── Bookmark Folder Types (#815) ─────────────────────────────────────────────

/// A user-owned folder for organizing bookmarks.
///
/// Folders are scoped per user (`owner`) and identified by a monotonically-
/// increasing `id` within the user's folder namespace (not global).  Each
/// folder stores a `Vec<u64>` of project IDs as its bookmark list under
/// `BookmarkKey::FolderBookmarks(owner, folder_id)`.
///
/// Nested folders are represented by an optional `parent_id`.  The depth limit
/// is enforced at creation time by `BookmarkRegistry`.  A `None` parent means
/// the folder is at the root level.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookmarkFolder {
    /// Folder identifier (monotonically increasing, scoped per user).
    pub id: u64,
    /// Address that owns this folder.
    pub owner: Address,
    /// Human-readable folder name (max `MAX_FOLDER_NAME_LEN` bytes).
    pub name: String,
    /// Optional parent folder ID.  `None` = root-level folder.
    pub parent_id: Option<u64>,
    /// Unix timestamp (seconds) when the folder was created.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the folder was last modified.
    pub updated_at: u64,
}

/// A smart folder that derives its bookmark list dynamically by applying a
/// `SmartFolderFilter` to the user's full bookmark set.  Smart folders are
/// read-only: bookmarks cannot be manually added or removed from them.
///
/// The smart folder record is stored but the resolved project-ID list is
/// computed on every `get_smart_folder_bookmarks` call (no caching).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartFolder {
    /// Smart folder identifier (scoped per user, separate counter from regular
    /// folders).
    pub id: u64,
    /// Address that owns this smart folder.
    pub owner: Address,
    /// Human-readable folder name.
    pub name: String,
    /// Filter criteria used to select bookmarks.
    pub filter: SmartFolderFilter,
    /// Unix timestamp (seconds) when the smart folder was created.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the smart folder was last modified.
    pub updated_at: u64,
}

/// Optional verification-status filter for a smart folder.
///
/// `None` is represented as `Any` — the filter matches regardless of
/// verification status.  Using a dedicated enum avoids the Soroban-SDK
/// limitation that prevents `Option<ContractType-enum>` from being used
/// inside another `#[contracttype]` struct.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationStatusFilter {
    /// No filter — match any verification status.
    Any,
    /// Match only projects with the given verification status.
    Is(VerificationStatus),
}

/// Filter criteria for a smart folder.
///
/// A smart folder resolves to the set of bookmarked projects that match **all**
/// of the non-`Any` fields (AND semantics).  Fields set to `None`/`Any` are
/// ignored.
///
/// Callers can create a "match-all" smart folder by setting every field to
/// `None` — this mirrors the full bookmark list.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartFolderFilter {
    /// Only include bookmarks for projects in this category, if set.
    pub category: Option<String>,
    /// Only include bookmarks for projects carrying this tag, if set.
    pub tag: Option<String>,
    /// Only include bookmarks for projects with this verification status
    /// (`VerificationStatusFilter::Is(...)`) or any status (`Any`).
    pub verification_status: VerificationStatusFilter,
}

/// The member list is stored separately under
/// `ExtensionKey::CollectionProjects(id)` as a `Vec<u64>` of project IDs.
/// The collection record itself only stores metadata.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collection {
    /// Unique monotonically-increasing collection identifier.
    pub id: u64,
    /// Human-readable collection name (max `MAX_COLLECTION_NAME_LEN` bytes).
    pub name: String,
    /// Short description of the collection's theme or curation criteria
    /// (max `MAX_COLLECTION_DESCRIPTION_LEN` bytes).
    pub description: String,
    /// Unix timestamp (seconds) when the collection was created.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the collection metadata was last updated.
    pub updated_at: u64,
}

/// Types of admin actions recorded in the admin action log.
///
/// **Variant-count ceiling:** Soroban encodes `#[contracttype]` enums as
/// `u32` discriminants, but XDR union types have a practical limit of
/// **50 variants** before SDK tooling and contract-size constraints become
/// problematic (same constraint as [`StorageKey`] / [`ExtensionKey`]).
/// This enum currently has **40 variants**; stay under 50. If you need
/// more, introduce an `AdminActionTypeExt` enum (mirroring the
/// `ExtensionKey` pattern) rather than pushing past the ceiling.
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

/// Current state of a duplicate-project dispute.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    /// Dispute has been submitted and is awaiting admin resolution.
    Pending,
    /// Admin determined the dispute was unfounded. **Terminal state.**
    Rejected,
    /// Admin resolved the dispute (e.g. archived the duplicate or linked
    /// the projects). **Terminal state.**
    Resolved,
}

/// A duplicate-project dispute raised by any user against a project they
/// believe is a duplicate of another.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateDispute {
    /// Unique monotonically-increasing dispute identifier.
    pub id: u64,
    /// ID of the project alleged to be a duplicate.
    pub project_id: u64,
    /// ID of the project alleged to be the original.
    pub original_project_id: u64,
    /// Address that opened the dispute.
    pub creator: Address,
    /// IPFS CID of the evidence document supplied by the creator.
    pub evidence_cid: String,
    /// Current state of the dispute (see [`DisputeStatus`]).
    pub status: DisputeStatus,
    /// Unix timestamp (seconds) when the dispute was opened.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the dispute was resolved or rejected.
    /// Zero while the dispute is still `Pending`.
    pub resolved_at: u64,
}

/// Action an admin takes when resolving a duplicate dispute.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisputeResolutionAction {
    /// Dispute is unfounded; no action taken on the projects.
    Reject,
    /// Archive the project with the given ID (typically the alleged duplicate).
    ArchiveProject(u64),
    /// Link the two projects as related rather than archiving either.
    LinkDuplicates,
}

/// A single entry in the admin action log.
///
/// Entries are appended by `AdminActionLog::record_action` after every
/// successful admin operation and are never mutated. Indexed under
/// `ExtensionKey::AdminActionLog` as a `Vec<AdminActionEntry>`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminActionEntry {
    /// Unique monotonically-increasing log entry identifier.
    pub id: u64,
    /// Admin address that performed the action.
    pub admin: Address,
    /// Type of admin action performed (see [`AdminActionType`]).
    pub action_type: AdminActionType,
    /// ID of the project, review, collection, or other entity affected by
    /// the action. `None` for actions that do not target a specific entity
    /// (e.g. `FeeChanged`, `ThresholdChanged`).
    pub target_id: Option<u64>,
    /// Address of the entity affected by the action (e.g. the new admin
    /// address for `AdminAdded`). `None` when not applicable.
    pub target_address: Option<Address>,
    /// Unix timestamp (seconds) when the action was recorded.
    pub timestamp: u64,
    /// Optional IPFS CID of a rationale or evidence document supplied at
    /// the time of the action (e.g. revocation reason, moderation note).
    pub reason_cid: Option<String>,
}

// ── Admin Timelock ───────────────────────────────────────────────────────────

/// A scheduled action in the admin timelock.
///
/// Created by `schedule_*` functions and executed by `execute_timelock_action`
/// after `execution_timestamp` has passed. The delay must be within
/// `[TIMELOCK_MIN_DELAY, TIMELOCK_MAX_DELAY]`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockAction {
    /// Unique monotonically-increasing timelock action identifier.
    pub id: u64,
    /// Admin address that scheduled the action.
    pub admin: Address,
    /// Type of action to be executed (see [`AdminActionType`]).
    pub action_type: AdminActionType,
    /// Unix timestamp (seconds) on or after which the action may be executed.
    pub execution_timestamp: u64,
    /// Whether the action has already been executed. Once `true`, the action
    /// cannot be executed again.
    pub executed: bool,
    /// Whether the action was cancelled before execution. Once `true`, the
    /// action cannot be executed or cancelled again.
    pub cancelled: bool,
    /// Unix timestamp (seconds) when the action was scheduled.
    pub created_at: u64,
}

/// Parameters for a scheduled fee change via timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockFeeParams {
    /// New token address (see `FeeConfig::token`).
    pub token: Option<Address>,
    /// New verification fee amount.
    pub verification_fee: u128,
    /// New registration fee amount.
    pub registration_fee: u128,
    /// New treasury address to receive fees.
    pub treasury: Address,
}

/// Parameters for a scheduled admin addition via timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockAdminAddParams {
    /// Address to be granted admin privileges.
    pub new_admin: Address,
}

/// Parameters for a scheduled admin removal via timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockAdminRemoveParams {
    /// Admin address to be removed.
    pub admin_to_remove: Address,
}

/// Lifecycle status of a multi-sig admin proposal.
///
/// ```text
/// Pending ──(threshold met)──► Approved ──execute──► Executed  (terminal)
///         └──(any admin rejects)──────────────────► Rejected   (terminal)
/// ```
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    /// Created and collecting approvals. Transitions to `Approved` once
    /// the required threshold is reached, or `Rejected` if cancelled.
    Pending,
    /// Approval threshold has been met; the proposal may now be executed.
    Approved,
    /// Proposal has been executed. **Terminal state.**
    Executed,
    /// Proposal was rejected before reaching the threshold. **Terminal state.**
    Rejected,
}

/// The operation encoded inside an `AdminProposal`.
///
/// The variant name and contained values are hashed at creation time
/// (`payload_hash`) and re-verified at execution time to prevent
/// payload-substitution attacks.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalPayload {
    /// Grant admin privileges to the enclosed address.
    AddAdmin(Address),
    /// Revoke admin privileges from the enclosed address.
    RemoveAdmin(Address),
    /// Update fee configuration: `(token, verification_fee, registration_fee, treasury)`.
    SetFee(Option<Address>, u128, u128, Address),
    /// Change the admin approval threshold to the enclosed value.
    SetThreshold(u32),
    /// Approve the verification request with the enclosed `request_id`.
    ApproveVerification(u64),
    /// Reject the verification request with the enclosed `request_id`.
    RejectVerification(u64),
    /// Revoke the verification for the project with the enclosed `project_id`,
    /// with the enclosed reason string.
    RevokeVerification(u64, String),
}

/// An admin proposal in the multi-sig workflow.
///
/// Proposals are created by any admin and collect approvals from other admins
/// until the configured threshold is met, at which point any admin may execute
/// the proposal. The `payload_hash` is a SHA-256 digest of the XDR-serialised
/// `payload` and is re-verified at execution time to prevent tampering.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminProposal {
    /// Unique monotonically-increasing proposal identifier.
    pub id: u64,
    /// Admin address that created the proposal.
    pub proposer: Address,
    /// High-level category of the proposed action (for logging and filtering).
    pub action_type: AdminActionType,
    /// SHA-256 digest of the XDR-encoded `payload`. Verified at execution
    /// time — a mismatch returns `PayloadHashMismatch`.
    pub payload_hash: soroban_sdk::BytesN<32>,
    /// Full operation parameters. Must hash to `payload_hash`.
    pub payload: ProposalPayload,
    /// Map of `admin_address → true` for each admin that has approved this
    /// proposal. An admin can only appear once; re-approving is a no-op.
    pub approvals: Map<Address, bool>,
    /// Current lifecycle status of the proposal (see [`ProposalStatus`]).
    pub status: ProposalStatus,
    /// Unix timestamp (seconds) when the proposal was created.
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
    /// ID of the project the deleted review belonged to.
    pub project_id: u64,
    /// Address of the reviewer who submitted (and deleted) the review.
    pub reviewer: Address,
    /// Unix timestamp (seconds) when the review was deleted.
    pub deleted_at: u64,
}

/// Compact on-chain record created when a review is archived by `archive_old_reviews`.
///
/// The full `Review` is removed from primary persistent storage (freeing storage
/// rent) and replaced by this lighter record stored at a shorter TTL. Off-chain
/// consumers that observe the `ReviewArchivedEvent` should persist the full
/// review payload to permanent storage (e.g., Arweave/IPFS) before the on-chain
/// archived record expires.
///
/// `get_archived_review` returns this type so callers can still query basic
/// review metadata and the Arweave/IPFS reference after archival.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivedReview {
    /// ID of the project the review belonged to.
    pub project_id: u64,
    /// Address of the reviewer.
    pub reviewer: Address,
    /// Rating at the time of archival (1–5).
    pub rating: u32,
    /// Canonical content CID pointing to off-chain review content.
    /// `None` when the original review had no off-chain content.
    pub content_cid: Option<String>,
    /// Unix timestamp (seconds) when the original review was submitted.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the review was last updated before archival.
    pub updated_at: u64,
    /// Unix timestamp (seconds) when this review was archived.
    pub archived_at: u64,
    /// Optional Arweave transaction ID set by an off-chain job after the full
    /// review payload has been written to permanent storage. `None` until an
    /// off-chain indexer calls `set_archived_review_arweave_tx`.
    pub arweave_tx_id: Option<String>,
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
    /// Maximum evidence links attachable to a single review (`MAX_EVIDENCE_LINKS_PER_REVIEW`).
    pub max_evidence_links_per_review: u32,
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

// ── Notification preference types (#811) ──────────────────────────────────────

/// How frequently the user wants to receive digest notifications.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DigestFrequency {
    /// No digest — immediate event-based notifications only.
    None,
    /// Receive a daily digest of queued project updates.
    Daily,
    /// Receive a weekly digest of queued project updates.
    Weekly,
}

/// The kind of project update that can trigger a notification.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationKind {
    /// The project's metadata was updated by its owner.
    ProjectUpdate,
    /// The project's verification was approved by an admin.
    VerificationApproved,
    /// The project's verification was rejected by an admin.
    VerificationRejected,
    /// The project's verification was revoked by an admin.
    VerificationRevoked,
    /// The project was archived.
    ProjectArchived,
    /// The project was reactivated from an archived state.
    ProjectReactivated,
}

/// Global notification preferences for a user.
///
/// Controls which project-update events are forwarded to the user
/// across all projects they follow, and whether digests are enabled.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserNotificationPrefs {
    /// Digest frequency preference.
    pub digest_frequency: DigestFrequency,
    /// When `true` the user wants notifications for every `NotificationKind`
    /// on every followed project (overrides `kinds`).
    pub notify_on_all: bool,
    /// When `true` the user has globally opted out and receives no notifications.
    pub opted_out: bool,
    /// Specific notification kinds the user wants when `notify_on_all` is false.
    pub kinds: Vec<NotificationKind>,
}

/// Per-project notification override for a specific user.
///
/// When present, overrides the user's global `UserNotificationPrefs`
/// for the given project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectNotificationOverride {
    /// When `true`, no notifications for this project regardless of global prefs.
    pub opted_out: bool,
    /// `Some(kinds)` overrides the user's global `kinds` list for this project.
    /// `None` means fall back to global preferences.
    pub kinds: Option<Vec<NotificationKind>>,
}

// ── Review content integrity types (#809) ────────────────────────────────────

/// Result of a review integrity verification check.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewIntegrityStatus {
    /// The review's current content matches the stored integrity seal.
    Valid,
    /// The review's current content does NOT match the stored seal.
    /// The on-chain data may have been tampered with or the seal is stale.
    Tampered,
    /// No integrity seal exists for this review (e.g. submitted before
    /// integrity sealing was enabled). The review is unverifiable.
    Unverifiable,
}

/// Stored integrity record for a single review.
///
/// Written at create / update time and read back by `verify_review_integrity`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewIntegrityRecord {
    /// SHA-256 seal over the canonical review payload at the time of last write.
    /// 32-byte SHA-256 digest stored as raw bytes.
    pub integrity_hash: soroban_sdk::Bytes,
    /// Ledger timestamp (seconds) when the seal was last written.
    pub sealed_at: u64,
    /// The content CID that was included in the sealed payload, if any.
    /// `None` means the review had no off-chain content when sealed.
    pub sealed_content_cid: Option<String>,
    /// The rating that was included in the sealed payload.
    pub sealed_rating: u32,
}
