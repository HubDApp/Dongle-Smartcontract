use crate::types::{
    AdminActionType, DigestFrequency, EvidenceLink, NotificationKind, ProjectLifecycleStatus,
    ReviewAction, ReviewEventData, VerificationStatus,
};
use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec};

pub const REVIEW: Symbol = symbol_short!("REVIEW");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperation {
    Verification,
    Registration,
}

// ── Event structs ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegisteredEvent {
    pub project_id: u64,
    pub owner: Address,
    pub name: String,
    pub category: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectLifecycleStatusUpdatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub previous_status: ProjectLifecycleStatus,
    pub new_status: ProjectLifecycleStatus,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationStatusResetEvent {
    pub project_id: u64,
    pub caller: Address,
    pub previous_status: VerificationStatus,
    pub new_status: VerificationStatus,
    pub fields: Vec<String>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectArchivedEvent {
    pub project_id: u64,
    pub archived_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReactivatedEvent {
    pub project_id: u64,
    pub caller: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectOwnershipTransferredEvent {
    pub project_id: u64,
    pub caller: Address,
    pub old_owner: Address,
    pub new_owner: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReportedEvent {
    pub project_id: u64,
    pub reporter: Address,
    pub reason_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReportsClearedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectTagsUpdatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub tags: Option<Vec<String>>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSocialLinksUpdatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub social_links: Option<Map<String, String>>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAddedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRemovedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewReportedEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub reporter: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewHiddenEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewRestoredEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewDeletedByAdminEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRequestedEvent {
    pub project_id: u64,
    pub requester: Address,
    pub evidence_cid: String,
    pub timestamp: u64,
    /// Id of the newly created `VerificationRecord` for this request.
    pub request_id: u64,
    /// Id of the previous verification request for this project, if any.
    /// `Some(_)` marks this request as a re-request (e.g. after rejection or
    /// revocation) rather than the project's first verification request.
    pub previous_request_id: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationApprovedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub decided_at: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRejectedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub decided_at: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationAppealSubmittedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub evidence_cid: String,
    pub appeal_count: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationAppealReviewedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub approved: bool,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRevokedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub reason: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationSuspendedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub reason: String,
    pub investigation_ticket: String,
    pub restore_at: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRestoredEvent {
    pub project_id: u64,
    pub admin: Option<Address>,
    pub timestamp: u64,
}

/// Emitted when a Verified project's expiry is checked and found to be expired.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationExpiredEvent {
    pub project_id: u64,
    pub expired_at: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationExpiryNotificationEvent {
    pub project_id: u64,
    pub owner: Address,
    pub expires_at: u64,
    pub renewal_instructions: String,
    pub sent_at: u64,
    pub resend: bool,
    pub resend_count: u32,
}

/// Emitted when an admin renews (resets the expiry of) a verified project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub new_expires_at: u64,
    pub timestamp: u64,
}

/// Emitted when project ownership is transferred.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationEvidenceUpdatedEvent {
    pub project_id: u64,
    pub requester: Address,
    pub old_evidence_cid: String,
    pub new_evidence_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationHistoryClearedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub removed_count: u32,
    pub retained_count: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenewalHistoryClearedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub removed_count: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewalReqEvent {
    pub project_id: u64,
    pub requester: Address,
    pub evidence_cid: String,
    pub fee_amount: u128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewalApprovedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub expires_at: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewalRejectedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinProjectAgeSetEvent {
    pub admin: Address,
    pub previous_min_age_seconds: u64,
    pub min_age_seconds: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationDurationSetEvent {
    pub admin: Address,
    pub previous_duration_seconds: u64,
    pub duration_seconds: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeSetEvent {
    pub admin: Address,
    pub token: Option<Address>,
    pub verification_fee: u128,
    pub registration_fee: u128,
    pub treasury: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaidEvent {
    pub project_id: u64,
    pub payer: Address,
    pub token: Option<Address>,
    pub operation: FeeOperation,
    pub amount: u128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConsumedEvent {
    pub project_id: u64,
    pub caller: Address,
    pub operation: FeeOperation,
    pub amount: u128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCancelledEvent {
    pub project_id: u64,
    pub caller: Address,
    pub payer: Address,
    pub operation: FeeOperation,
    pub amount: u128,
    pub timestamp: u64,
}

// ── Publish helpers ───────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn publish_review_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    action: ReviewAction,
    content_cid: Option<String>,
    owner_response: Option<String>,
    created_at: u64,
    updated_at: u64,
    evidence_links: Vec<EvidenceLink>,
) {
    let event_data = ReviewEventData {
        project_id,
        reviewer: reviewer.clone(),
        action: action.clone(),
        timestamp: env.ledger().timestamp(),
        content_cid,
        created_at,
        updated_at,
        owner_response,
        evidence_links,
    };

    let action_sym = match action {
        ReviewAction::Submitted => symbol_short!("SUBMITTED"),
        ReviewAction::Updated => symbol_short!("UPDATED"),
        ReviewAction::Revised => symbol_short!("REVISED"),
        ReviewAction::Deleted => symbol_short!("DELETED"),
    };

    env.events()
        .publish((REVIEW, action_sym, project_id, reviewer), event_data);
}

#[allow(clippy::too_many_arguments)]
pub fn publish_review_revision_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    revision_index: u32,
    previous_rating: u32,
    previous_content_cid: Option<String>,
    new_rating: u32,
    new_content_cid: Option<String>,
) {
    use crate::types::ReviewRevisionEvent;

    let event_data = ReviewRevisionEvent {
        project_id,
        reviewer: reviewer.clone(),
        revision_index,
        previous_rating,
        previous_content_cid,
        new_rating,
        new_content_cid,
        timestamp: env.ledger().timestamp(),
    };

    env.events().publish(
        (REVIEW, symbol_short!("REVISED"), project_id, reviewer),
        event_data,
    );
}

pub fn publish_project_registered_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    name: String,
    category: String,
) {
    let event_data = ProjectRegisteredEvent {
        project_id,
        owner,
        name,
        category,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("CREATED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_updated_event(env: &Env, project_id: u64, owner: Address) {
    let event_data = ProjectUpdatedEvent {
        project_id,
        owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UPDATED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_lifecycle_status_updated_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    previous_status: ProjectLifecycleStatus,
    new_status: ProjectLifecycleStatus,
) {
    let event_data = ProjectLifecycleStatusUpdatedEvent {
        project_id,
        owner,
        previous_status,
        new_status,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("LCSCHED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_verification_status_reset_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    previous_status: VerificationStatus,
    fields: Vec<String>,
) {
    let event_data = VerificationStatusResetEvent {
        project_id,
        caller,
        previous_status,
        new_status: VerificationStatus::Unverified,
        fields,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("RESET"), project_id),
        event_data,
    );
}

pub fn publish_project_archived_event(env: &Env, project_id: u64, archived_by: Address) {
    let event_data = ProjectArchivedEvent {
        project_id,
        archived_by,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("ARCHIVED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_reactivated_event(env: &Env, project_id: u64, caller: Address) {
    let event_data = ProjectReactivatedEvent {
        project_id,
        caller,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("RESTORED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_reported_event(
    env: &Env,
    project_id: u64,
    reporter: Address,
    reason_cid: String,
) {
    let event_data = ProjectReportedEvent {
        project_id,
        reporter,
        reason_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("REPORTED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_reports_cleared_event(env: &Env, project_id: u64, admin: Address) {
    let event_data = ProjectReportsClearedEvent {
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("RPCLEARED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_tags_updated_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    tags: Option<Vec<String>>,
) {
    let event_data = ProjectTagsUpdatedEvent {
        project_id,
        owner,
        tags,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("PROJECT"), symbol_short!("TAGS"), project_id),
        event_data,
    );
}

pub fn publish_project_social_links_updated_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    social_links: Option<Map<String, String>>,
) {
    let event_data = ProjectSocialLinksUpdatedEvent {
        project_id,
        owner,
        social_links,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("SOCIAL"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_verification_expired_event(env: &Env, project_id: u64, expired_at: u64) {
    let now = env.ledger().timestamp();
    let event_data = VerificationExpiredEvent {
        project_id,
        expired_at,
        timestamp: now,
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("EXPRD"), project_id),
        event_data,
    );
}

pub fn publish_verification_expiry_notification_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    expires_at: u64,
    renewal_instructions: String,
    resend: bool,
    resend_count: u32,
) {
    let event_data = VerificationExpiryNotificationEvent {
        project_id,
        owner: owner.clone(),
        expires_at,
        renewal_instructions,
        sent_at: env.ledger().timestamp(),
        resend,
        resend_count,
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REMINDER"), project_id, owner),
        event_data,
    );
}

pub fn publish_verification_renewed_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    new_expires_at: u64,
) {
    let event_data = VerificationRenewedEvent {
        project_id,
        admin,
        new_expires_at,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("RENEWD"), project_id),
        event_data,
    );
}

// ── Admin events ──────────────────────────────────────────────────────────────

pub fn publish_ownership_transferred_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    old_owner: Address,
    new_owner: Address,
) {
    let event_data = ProjectOwnershipTransferredEvent {
        project_id,
        caller,
        old_owner,
        new_owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("TRANSFER"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_admin_added_event(env: &Env, admin: Address) {
    let event_data = AdminAddedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("ADMIN"), symbol_short!("ADDED")), event_data);
}

pub fn publish_admin_removed_event(env: &Env, admin: Address) {
    let event_data = AdminRemovedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("ADMIN"), symbol_short!("REMOVED")),
        event_data,
    );
}

pub fn publish_review_reported_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    reporter: Address,
) {
    let event_data = ReviewReportedEvent {
        project_id,
        reviewer,
        reporter,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("REVIEW"),
            symbol_short!("REPORTED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_review_hidden_event(env: &Env, project_id: u64, reviewer: Address, admin: Address) {
    let event_data = ReviewHiddenEvent {
        project_id,
        reviewer,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("REVIEW"), symbol_short!("HIDDEN"), project_id),
        event_data,
    );
}

pub fn publish_review_restored_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    admin: Address,
) {
    let event_data = ReviewRestoredEvent {
        project_id,
        reviewer,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("REVIEW"),
            symbol_short!("RESTORED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_review_deleted_by_admin_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    admin: Address,
) {
    let event_data = ReviewDeletedByAdminEvent {
        project_id,
        reviewer,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("REVIEW"),
            symbol_short!("ADMINDEL"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_verification_requested_event(
    env: &Env,
    project_id: u64,
    requester: Address,
    evidence_cid: String,
    request_id: u64,
    previous_request_id: Option<u64>,
) {
    let event_data = VerificationRequestedEvent {
        project_id,
        requester,
        evidence_cid,
        timestamp: env.ledger().timestamp(),
        request_id,
        previous_request_id,
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REQ"), project_id),
        event_data,
    );
}

pub fn publish_verification_approved_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    decided_at: u64,
) {
    let event_data = VerificationApprovedEvent {
        project_id,
        admin,
        decided_at,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("APP"), project_id),
        event_data,
    );
}

pub fn publish_verification_rejected_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    decided_at: u64,
) {
    let event_data = VerificationRejectedEvent {
        project_id,
        admin,
        decided_at,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REJ"), project_id),
        event_data,
    );
}

pub fn publish_verification_appeal_submitted_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    evidence_cid: String,
    appeal_count: u32,
) {
    let event_data = VerificationAppealSubmittedEvent {
        project_id,
        owner,
        evidence_cid,
        appeal_count,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("APPEAL"), project_id),
        event_data,
    );
}

pub fn publish_verification_appeal_reviewed_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    approved: bool,
) {
    let event_data = VerificationAppealReviewedEvent {
        project_id,
        admin,
        approved,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), if approved { symbol_short!("APPRV") } else { symbol_short!("APDENY") }, project_id),
        event_data,
    );
}

pub fn publish_verification_revoked_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    reason: String,
) {
    let event_data = VerificationRevokedEvent {
        project_id,
        admin,
        reason,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("VERIFY"),
            symbol_short!("REVOKED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_verification_suspended_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    reason: String,
    investigation_ticket: String,
    restore_at: u64,
) {
    let event_data = VerificationSuspendedEvent {
        project_id,
        admin: admin.clone(),
        reason,
        investigation_ticket,
        restore_at,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("SUSPENDED"), project_id),
        event_data,
    );
}

pub fn publish_verification_restored_event(
    env: &Env,
    project_id: u64,
    admin: Option<Address>,
) {
    let event_data = VerificationRestoredEvent {
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("RESTORED"), project_id),
        event_data,
    );
}

pub fn publish_verification_evidence_updated_event(
    env: &Env,
    project_id: u64,
    requester: Address,
    old_evidence_cid: String,
    new_evidence_cid: String,
) {
    let event_data = VerificationEvidenceUpdatedEvent {
        project_id,
        requester,
        old_evidence_cid,
        new_evidence_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("EV_UPD"), project_id),
        event_data,
    );
}

pub fn publish_verification_history_cleared_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    removed_count: u32,
    retained_count: u32,
) {
    let event_data = VerificationHistoryClearedEvent {
        project_id,
        admin,
        removed_count,
        retained_count,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("VERIFY"),
            symbol_short!("HISTCLR"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_renewal_history_cleared_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    removed_count: u32,
) {
    let event_data = RenewalHistoryClearedEvent {
        project_id,
        admin,
        removed_count,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("RENEW"), symbol_short!("HISTCLR"), project_id),
        event_data,
    );
}

pub fn publish_verification_renewal_requested_event(
    env: &Env,
    project_id: u64,
    requester: Address,
    evidence_cid: String,
    fee_amount: u128,
) {
    let event_data = VerificationRenewalReqEvent {
        project_id,
        requester,
        evidence_cid,
        fee_amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("RENEW"), symbol_short!("REQUEST"), project_id),
        event_data,
    );
}

pub fn publish_verification_renewal_approved_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    expires_at: u64,
) {
    let event_data = VerificationRenewalApprovedEvent {
        project_id,
        admin,
        expires_at,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("RENEW"),
            symbol_short!("APPROVED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_verification_renewal_rejected_event(env: &Env, project_id: u64, admin: Address) {
    let event_data = VerificationRenewalRejectedEvent {
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("RENEW"),
            symbol_short!("REJECTED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_fee_paid_event(
    env: &Env,
    project_id: u64,
    payer: Address,
    token: Option<Address>,
    operation: FeeOperation,
    amount: u128,
) {
    let event_data = FeePaidEvent {
        project_id,
        payer,
        token,
        operation,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("PAID"), project_id),
        event_data,
    );
}

pub fn publish_fee_consumed_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    operation: FeeOperation,
    amount: u128,
) {
    let event_data = FeeConsumedEvent {
        project_id,
        caller,
        operation,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("CONSUMED"), project_id),
        event_data,
    );
}

pub fn publish_fee_cancelled_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    payer: Address,
    operation: FeeOperation,
    amount: u128,
) {
    let event_data = FeeCancelledEvent {
        project_id,
        caller,
        payer,
        operation,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("CANCEL"), project_id),
        event_data,
    );
}

pub fn publish_fee_set_event(
    env: &Env,
    admin: Address,
    token: Option<Address>,
    verification_fee: u128,
    registration_fee: u128,
    treasury: Address,
) {
    let event_data = FeeSetEvent {
        admin,
        token,
        verification_fee,
        registration_fee,
        treasury,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("CONFIG"), symbol_short!("FEE")), event_data);
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReviewsEnabledSetEvent {
    pub project_id: u64,
    pub caller: Address,
    pub enabled: bool,
    pub timestamp: u64,
}

pub fn publish_project_reviews_enabled_set_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    enabled: bool,
) {
    let event_data = ProjectReviewsEnabledSetEvent {
        project_id,
        caller,
        enabled,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("REVIEWS"),
            project_id,
        ),
        event_data,
    );
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectClaimableSetEvent {
    pub project_id: u64,
    pub caller: Address,
    pub claimable: bool,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRequestSubmittedEvent {
    pub claim_request_id: u64,
    pub project_id: u64,
    pub claimant: Address,
    pub proof_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRequestApprovedEvent {
    pub claim_request_id: u64,
    pub project_id: u64,
    pub claimant: Address,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRequestRejectedEvent {
    pub claim_request_id: u64,
    pub project_id: u64,
    pub claimant: Address,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaimSubmittedEvent {
    pub project_id: u64,
    pub contract_address: String,
    pub claimant: Address,
    pub proof_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaimApprovedEvent {
    pub project_id: u64,
    pub contract_address: String,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClaimRejectedEvent {
    pub project_id: u64,
    pub contract_address: String,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_project_claimable_set_event(
    env: &Env,
    project_id: u64,
    caller: Address,
    claimable: bool,
) {
    let event_data = ProjectClaimableSetEvent {
        project_id,
        caller,
        claimable,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("CLAIMABLE"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_claim_request_submitted_event(
    env: &Env,
    claim_request_id: u64,
    project_id: u64,
    claimant: Address,
    proof_cid: String,
) {
    let event_data = ClaimRequestSubmittedEvent {
        claim_request_id,
        project_id,
        claimant: claimant.clone(),
        proof_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CLAIM"),
            symbol_short!("SUBMITTED"),
            project_id,
            claimant,
        ),
        event_data,
    );
}

pub fn publish_claim_request_approved_event(
    env: &Env,
    claim_request_id: u64,
    project_id: u64,
    claimant: Address,
    admin: Address,
) {
    let event_data = ClaimRequestApprovedEvent {
        claim_request_id,
        project_id,
        claimant: claimant.clone(),
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CLAIM"),
            symbol_short!("APPROVED"),
            project_id,
            claimant,
        ),
        event_data,
    );
}

pub fn publish_claim_request_rejected_event(
    env: &Env,
    claim_request_id: u64,
    project_id: u64,
    claimant: Address,
    admin: Address,
) {
    let event_data = ClaimRequestRejectedEvent {
        claim_request_id,
        project_id,
        claimant: claimant.clone(),
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CLAIM"),
            symbol_short!("REJECTED"),
            project_id,
            claimant,
        ),
        event_data,
    );
}

pub fn publish_contract_claim_submitted_event(
    env: &Env,
    project_id: u64,
    contract_address: String,
    claimant: Address,
    proof_cid: String,
) {
    let event_data = ContractClaimSubmittedEvent {
        project_id,
        contract_address: contract_address.clone(),
        claimant: claimant.clone(),
        proof_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CCLAIM"),
            symbol_short!("SUBMITTED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_contract_claim_approved_event(
    env: &Env,
    project_id: u64,
    contract_address: String,
    admin: Address,
) {
    let event_data = ContractClaimApprovedEvent {
        project_id,
        contract_address: contract_address.clone(),
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CCLAIM"),
            symbol_short!("APPROVED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_contract_claim_rejected_event(
    env: &Env,
    project_id: u64,
    contract_address: String,
    admin: Address,
) {
    let event_data = ContractClaimRejectedEvent {
        project_id,
        contract_address: contract_address.clone(),
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CCLAIM"),
            symbol_short!("REJECTED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_min_project_age_set_event(
    env: &Env,
    admin: Address,
    previous_min_age_seconds: u64,
    min_age_seconds: u64,
) {
    let event_data = MinProjectAgeSetEvent {
        admin,
        previous_min_age_seconds,
        min_age_seconds,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONFIG"), symbol_short!("MIN_AGE")),
        event_data,
    );
}

pub fn publish_verification_duration_set_event(
    env: &Env,
    admin: Address,
    previous_duration_seconds: u64,
    duration_seconds: u64,
) {
    let event_data = VerificationDurationSetEvent {
        admin,
        previous_duration_seconds,
        duration_seconds,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONFIG"), symbol_short!("DURATION")),
        event_data,
    );
}

pub fn publish_featured_project_event(env: &Env, project_id: u64, featured: bool, admin: Address) {
    let event_data = crate::types::FeaturedProjectEvent {
        project_id,
        featured,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("FEATURED"),
            project_id,
        ),
        event_data,
    );
}

// ── Collection Events ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionCreatedEvent {
    pub collection_id: u64,
    pub name: String,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionUpdatedEvent {
    pub collection_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionDeletedEvent {
    pub collection_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectAddedToCollectionEvent {
    pub collection_id: u64,
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjRemovedFromCollectionEvent {
    pub collection_id: u64,
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_collection_created_event(
    env: &Env,
    collection_id: u64,
    name: String,
    admin: Address,
) {
    let event_data = CollectionCreatedEvent {
        collection_id,
        name,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("CREATED"),
            collection_id,
        ),
        event_data,
    );
}

pub fn publish_collection_updated_event(env: &Env, collection_id: u64, admin: Address) {
    let event_data = CollectionUpdatedEvent {
        collection_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("UPDATED"),
            collection_id,
        ),
        event_data,
    );
}

pub fn publish_collection_deleted_event(env: &Env, collection_id: u64, admin: Address) {
    let event_data = CollectionDeletedEvent {
        collection_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("DELETED"),
            collection_id,
        ),
        event_data,
    );
}

pub fn publish_project_added_to_collection_event(
    env: &Env,
    collection_id: u64,
    project_id: u64,
    admin: Address,
) {
    let event_data = ProjectAddedToCollectionEvent {
        collection_id,
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("ADDED"),
            collection_id,
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_removed_from_collection_event(
    env: &Env,
    collection_id: u64,
    project_id: u64,
    admin: Address,
) {
    let event_data = ProjRemovedFromCollectionEvent {
        collection_id,
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("COLLECT"),
            symbol_short!("REMOVED"),
            collection_id,
            project_id,
        ),
        event_data,
    );
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectLinkedEvent {
    pub project_id: u64,
    pub linked_project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnlinkedEvent {
    pub project_id: u64,
    pub linked_project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

pub fn publish_project_linked_event(
    env: &Env,
    project_id: u64,
    linked_project_id: u64,
    owner: Address,
) {
    let event_data = ProjectLinkedEvent {
        project_id,
        linked_project_id,
        owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("LINKED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_unlinked_event(
    env: &Env,
    project_id: u64,
    linked_project_id: u64,
    owner: Address,
) {
    let event_data = ProjectUnlinkedEvent {
        project_id,
        linked_project_id,
        owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNLINKED"),
            project_id,
        ),
        event_data,
    );
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateDisputeOpenedEvent {
    pub dispute_id: u64,
    pub project_id: u64,
    pub original_project_id: u64,
    pub creator: Address,
    pub evidence_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateDisputeResolvedEvent {
    pub dispute_id: u64,
    pub admin: Address,
    pub action: crate::types::DisputeResolutionAction,
    pub timestamp: u64,
}

pub fn publish_duplicate_dispute_opened_event(
    env: &Env,
    dispute_id: u64,
    project_id: u64,
    original_project_id: u64,
    creator: Address,
    evidence_cid: String,
) {
    let event_data = DuplicateDisputeOpenedEvent {
        dispute_id,
        project_id,
        original_project_id,
        creator: creator.clone(),
        evidence_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("DISPUTE"),
            symbol_short!("OPENED"),
            project_id,
            creator,
        ),
        event_data,
    );
}

pub fn publish_duplicate_dispute_resolved_event(
    env: &Env,
    dispute_id: u64,
    admin: Address,
    action: crate::types::DisputeResolutionAction,
) {
    let event_data = DuplicateDisputeResolvedEvent {
        dispute_id,
        admin: admin.clone(),
        action,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("DISPUTE"),
            symbol_short!("RESOLVED"),
            dispute_id,
            admin,
        ),
        event_data,
    );
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectMaintainerAddedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub maintainer: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectMaintainerRemovedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub maintainer: Address,
    pub timestamp: u64,
}

pub fn publish_project_maintainer_added_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    maintainer: Address,
) {
    let event_data = ProjectMaintainerAddedEvent {
        project_id,
        owner,
        maintainer: maintainer.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("M_ADDED"),
            project_id,
            maintainer,
        ),
        event_data,
    );
}

pub fn publish_project_maintainer_removed_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    maintainer: Address,
) {
    let event_data = ProjectMaintainerRemovedEvent {
        project_id,
        owner,
        maintainer: maintainer.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("M_REMOVED"),
            project_id,
            maintainer,
        ),
        event_data,
    );
}

// ── Subscription / Follow Events ─────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectFollowedEvent {
    pub project_id: u64,
    pub follower: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnfollowedEvent {
    pub project_id: u64,
    pub follower: Address,
    pub timestamp: u64,
}

pub fn publish_project_followed_event(env: &Env, project_id: u64, follower: Address) {
    let event_data = ProjectFollowedEvent {
        project_id,
        follower: follower.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("FOLLOWED"),
            project_id,
            follower,
        ),
        event_data,
    );
}

// ── Timelock Events ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockActionScheduledEvent {
    pub action_id: u64,
    pub admin: Address,
    pub action_type: AdminActionType,
    pub execution_timestamp: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockActionCancelledEvent {
    pub action_id: u64,
    pub admin: Address,
    pub action_type: AdminActionType,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelockActionExecutedEvent {
    pub action_id: u64,
    pub admin: Address,
    pub action_type: AdminActionType,
    pub timestamp: u64,
}

pub fn publish_timelock_action_scheduled_event(
    env: &Env,
    action_id: u64,
    admin: Address,
    action_type: AdminActionType,
    execution_timestamp: u64,
) {
    let event_data = TimelockActionScheduledEvent {
        action_id,
        admin,
        action_type,
        execution_timestamp,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("TIMELOCK"), symbol_short!("SCHEDULE")),
        event_data,
    );
}

pub fn publish_timelock_action_cancelled_event(
    env: &Env,
    action_id: u64,
    admin: Address,
    action_type: AdminActionType,
) {
    let event_data = TimelockActionCancelledEvent {
        action_id,
        admin,
        action_type,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("TIMELOCK"), symbol_short!("CANCEL")),
        event_data,
    );
}

pub fn publish_timelock_action_executed_event(
    env: &Env,
    action_id: u64,
    admin: Address,
    action_type: AdminActionType,
) {
    let event_data = TimelockActionExecutedEvent {
        action_id,
        admin,
        action_type,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("TIMELOCK"), symbol_short!("EXECUTE")),
        event_data,
    );
}

// ── Bookmark Events ──────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectBookmarkedEvent {
    pub project_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnbookmarkedEvent {
    pub project_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

pub fn publish_project_bookmarked_event(env: &Env, project_id: u64, user: Address) {
    let event_data = ProjectBookmarkedEvent {
        project_id,
        user: user.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("BOOKMARK"),
            project_id,
            user,
        ),
        event_data,
    );
}

pub fn publish_project_unbookmarked_event(env: &Env, project_id: u64, user: Address) {
    let event_data = ProjectUnbookmarkedEvent {
        project_id,
        user: user.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNBOOKMK"),
            project_id,
            user,
        ),
        event_data,
    );
}

pub fn publish_project_unfollowed_event(env: &Env, project_id: u64, follower: Address) {
    let event_data = ProjectUnfollowedEvent {
        project_id,
        follower: follower.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNFOLLOW"),
            project_id,
            follower,
        ),
        event_data,
    );
}

// ── Endorsement Events ─────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectEndorsedEvent {
    pub project_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnendorsedEvent {
    pub project_id: u64,
    pub user: Address,
    pub timestamp: u64,
}

pub fn publish_project_endorsed_event(env: &Env, project_id: u64, user: Address) {
    let event_data = ProjectEndorsedEvent {
        project_id,
        user: user.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("ENDORSE"),
            project_id,
            user,
        ),
        event_data,
    );
}

pub fn publish_project_unendorsed_event(env: &Env, project_id: u64, user: Address) {
    let event_data = ProjectUnendorsedEvent {
        project_id,
        user: user.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNENDOR"),
            project_id,
            user,
        ),
        event_data,
    );
}

// ── Fee Refund / Expiry Events ─────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeRefundedEvent {
    pub project_id: u64,
    pub request_id: u64,
    pub payer: Address,
    pub amount: u128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaymentClearedEvent {
    pub project_id: u64,
    pub payer: Address,
    pub paid_at: u64,
    pub cleared_at: u64,
}

pub fn publish_fee_refunded_event(
    env: &Env,
    project_id: u64,
    request_id: u64,
    payer: Address,
    amount: u128,
) {
    let event_data = FeeRefundedEvent {
        project_id,
        request_id,
        payer,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("REFUNDED"), project_id),
        event_data,
    );
}

// ── Verification Assignment Events ─────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationAssignedEvent {
    pub project_id: u64,
    pub request_id: u64,
    pub assigned_admin: Address,
    pub assigner: Address,
    pub timestamp: u64,
}

pub fn publish_verification_assigned_event(
    env: &Env,
    project_id: u64,
    request_id: u64,
    assigned_admin: Address,
    assigner: Address,
) {
    let event_data = VerificationAssignedEvent {
        project_id,
        request_id,
        assigned_admin,
        assigner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("VERIFY"),
            symbol_short!("ASSIGNED"),
            project_id,
        ),
        event_data,
    );
}

// ── Reserved Name Events ──────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReservedNameAddedEvent {
    pub name: String,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReservedNameRemovedEvent {
    pub name: String,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_reserved_name_added_event(env: &Env, name: String, admin: Address) {
    let event_data = ReservedNameAddedEvent {
        name,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONFIG"), symbol_short!("RSVD_ADD")),
        event_data,
    );
}

pub fn publish_reserved_name_removed_event(env: &Env, name: String, admin: Address) {
    let event_data = ReservedNameRemovedEvent {
        name,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONFIG"), symbol_short!("RSVD_REM")),
        event_data,
    );
}

// ── Project Changelog Events ──────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangelogAddedEvent {
    pub changelog_id: u64,
    pub project_id: u64,
    pub owner: Address,
    pub cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangelogRemovedEvent {
    pub changelog_id: u64,
    pub project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

// ── Contract Pause / Emergency Stop Events ─────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractPausedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUnpausedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_contract_paused_event(env: &Env, admin: Address) {
    let event_data = ContractPausedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONTRACT"), symbol_short!("PAUSED")),
        event_data,
    );
}

pub fn publish_contract_unpaused_event(env: &Env, admin: Address) {
    let event_data = ContractUnpausedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("CONTRACT"), symbol_short!("UNPAUSED")),
        event_data,
    );
}

pub fn publish_fee_payment_cleared_event(
    env: &Env,
    project_id: u64,
    payer: Address,
    paid_at: u64,
    cleared_at: u64,
) {
    let event_data = FeePaymentClearedEvent {
        project_id,
        payer,
        paid_at,
        cleared_at,
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("CLEARED"), project_id),
        event_data,
    );
}

// ── Changelog Event Functions ───────────────────────────────────────

pub fn publish_changelog_added_event(
    env: &Env,
    changelog_id: u64,
    project_id: u64,
    owner: Address,
    cid: String,
) {
    let event_data = ChangelogAddedEvent {
        changelog_id,
        project_id,
        owner,
        cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CHANGELOG"),
            symbol_short!("ADDED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_changelog_removed_event(
    env: &Env,
    changelog_id: u64,
    project_id: u64,
    owner: Address,
) {
    let event_data = ChangelogRemovedEvent {
        changelog_id,
        project_id,
        owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("CHANGELOG"),
            symbol_short!("REMOVED"),
            project_id,
        ),
        event_data,
    );
}

// ── Notification Events (#811) ────────────────────────────────────────────────

/// Emitted on a project update that followers should be notified about.
///
/// Indexers subscribe to `(PROJECT, NOTIF, project_id)` topics to fan out
/// the notification to each follower. The `follower_count` field allows the
/// indexer to allocate its fanout work without a separate chain read.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdateNotificationEvent {
    /// The project that was updated.
    pub project_id: u64,
    /// What kind of update occurred.
    pub update_kind: NotificationKind,
    /// Cached follower count at the time of the event.
    pub follower_count: u32,
    /// Ledger timestamp when the event was emitted.
    pub timestamp: u64,
}

/// Emitted when a user's digest queue is flushed and a digest is scheduled
/// for delivery.
///
/// Off-chain services consume this event to build and send the actual
/// digest message (email, push, etc.).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserDigestScheduledEvent {
    /// The user whose digest is being dispatched.
    pub user: Address,
    /// Project IDs included in this digest batch.
    pub queued_project_ids: Vec<u64>,
    /// The frequency that triggered this digest.
    pub frequency: DigestFrequency,
    /// Ledger timestamp when the digest was scheduled.
    pub timestamp: u64,
}

/// Emitted when a user updates their notification preferences.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationPrefsUpdatedEvent {
    pub user: Address,
    pub opted_out: bool,
    pub digest_frequency: DigestFrequency,
    pub timestamp: u64,
}

/// Emitted when a user sets a per-project notification override.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectNotifOverrideSetEvent {
    pub user: Address,
    pub project_id: u64,
    pub opted_out: bool,
    pub timestamp: u64,
}

pub fn publish_project_update_notification_event(
    env: &Env,
    project_id: u64,
    update_kind: NotificationKind,
    follower_count: u32,
) {
    let event_data = ProjectUpdateNotificationEvent {
        project_id,
        update_kind: update_kind.clone(),
        follower_count,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("NOTIF"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_user_digest_scheduled_event(
    env: &Env,
    user: Address,
    queued_project_ids: Vec<u64>,
    frequency: DigestFrequency,
) {
    let event_data = UserDigestScheduledEvent {
        user: user.clone(),
        queued_project_ids,
        frequency,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("USER"),
            symbol_short!("DIGEST"),
            user,
        ),
        event_data,
    );
}

pub fn publish_notification_prefs_updated_event(
    env: &Env,
    user: Address,
    opted_out: bool,
    digest_frequency: DigestFrequency,
) {
    let event_data = NotificationPrefsUpdatedEvent {
        user: user.clone(),
        opted_out,
        digest_frequency,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("USER"),
            symbol_short!("NFPREF"),
            user,
        ),
        event_data,
    );
}

pub fn publish_project_notif_override_set_event(
    env: &Env,
    user: Address,
    project_id: u64,
    opted_out: bool,
) {
    let event_data = ProjectNotifOverrideSetEvent {
        user: user.clone(),
        project_id,
        opted_out,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("NFOVRRD"),
            project_id,
            user,
        ),
        event_data,
    );
}

// ── Review Content Integrity Events (#809) ────────────────────────────────────

/// Emitted when a review integrity seal is written (on create or update).
///
/// Indexers can subscribe to `(REVIEW, SEALED, project_id)` to track seal history.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewIntegritySealedEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub sealed_rating: u32,
    pub has_content_cid: bool,
    pub timestamp: u64,
}

/// Emitted by `verify_review_integrity` when the live review content
/// does NOT match its stored seal — indicating possible tampering.
///
/// Off-chain monitoring tools should alert on this event.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewIntegrityViolationEvent {
    pub project_id: u64,
    pub reviewer: Address,
    /// The rating currently stored on-chain.
    pub current_rating: u32,
    /// The rating that was present when the seal was written.
    pub sealed_rating: u32,
    /// Whether the current on-chain review has a content CID.
    pub current_has_cid: bool,
    /// Whether the sealed snapshot had a content CID.
    pub sealed_has_cid: bool,
    pub timestamp: u64,
}

pub fn publish_review_integrity_sealed_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    sealed_rating: u32,
    has_content_cid: bool,
) {
    let event_data = ReviewIntegritySealedEvent {
        project_id,
        reviewer: reviewer.clone(),
        sealed_rating,
        has_content_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("REVIEW"),
            symbol_short!("SEALED"),
            project_id,
            reviewer,
        ),
        event_data,
    );
}

pub fn publish_review_integrity_violation_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    current_rating: u32,
    sealed_rating: u32,
    current_has_cid: bool,
    sealed_has_cid: bool,
) {
    let event_data = ReviewIntegrityViolationEvent {
        project_id,
        reviewer: reviewer.clone(),
        current_rating,
        sealed_rating,
        current_has_cid,
        sealed_has_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("REVIEW"),
            symbol_short!("TAMPER"),
            project_id,
            reviewer,
        ),
        event_data,
    );
}

// ── Bookmark Folder Events (#815) ─────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderCreatedEvent {
    pub folder_id: u64,
    pub owner: Address,
    pub name: String,
    pub parent_id: Option<u64>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderDeletedEvent {
    pub folder_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderRenamedEvent {
    pub folder_id: u64,
    pub owner: Address,
    pub new_name: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookmarkMovedToFolderEvent {
    pub project_id: u64,
    pub owner: Address,
    pub folder_id: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookmarkRemovedFromFolderEvent {
    pub project_id: u64,
    pub owner: Address,
    pub folder_id: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartFolderCreatedEvent {
    pub smart_folder_id: u64,
    pub owner: Address,
    pub name: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartFolderDeletedEvent {
    pub smart_folder_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

pub fn publish_folder_created_event(
    env: &Env,
    folder_id: u64,
    owner: Address,
    name: String,
    parent_id: Option<u64>,
) {
    let event_data = FolderCreatedEvent {
        folder_id,
        owner: owner.clone(),
        name,
        parent_id,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("FOLDER"),
            symbol_short!("CREATED"),
            folder_id,
            owner,
        ),
        event_data,
    );
}

pub fn publish_folder_deleted_event(env: &Env, folder_id: u64, owner: Address) {
    let event_data = FolderDeletedEvent {
        folder_id,
        owner: owner.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("FOLDER"),
            symbol_short!("DELETED"),
            folder_id,
            owner,
        ),
        event_data,
    );
}

pub fn publish_folder_renamed_event(env: &Env, folder_id: u64, owner: Address, new_name: String) {
    let event_data = FolderRenamedEvent {
        folder_id,
        owner: owner.clone(),
        new_name,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("FOLDER"),
            symbol_short!("RENAMED"),
            folder_id,
            owner,
        ),
        event_data,
    );
}

pub fn publish_bookmark_moved_to_folder_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    folder_id: u64,
) {
    let event_data = BookmarkMovedToFolderEvent {
        project_id,
        owner: owner.clone(),
        folder_id,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("BOOKMARK"),
            symbol_short!("MOVED"),
            project_id,
            owner,
        ),
        event_data,
    );
}

pub fn publish_bookmark_removed_from_folder_event(
    env: &Env,
    project_id: u64,
    owner: Address,
    folder_id: u64,
) {
    let event_data = BookmarkRemovedFromFolderEvent {
        project_id,
        owner: owner.clone(),
        folder_id,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("BOOKMARK"),
            symbol_short!("RMVDFDR"),
            project_id,
            owner,
        ),
        event_data,
    );
}

pub fn publish_smart_folder_created_event(
    env: &Env,
    smart_folder_id: u64,
    owner: Address,
    name: String,
) {
    let event_data = SmartFolderCreatedEvent {
        smart_folder_id,
        owner: owner.clone(),
        name,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("SFOLDER"),
            symbol_short!("CREATED"),
            smart_folder_id,
            owner,
        ),
        event_data,
    );
}

pub fn publish_smart_folder_deleted_event(env: &Env, smart_folder_id: u64, owner: Address) {
    let event_data = SmartFolderDeletedEvent {
        smart_folder_id,
        owner: owner.clone(),
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("SFOLDER"),
            symbol_short!("DELETED"),
            smart_folder_id,
            owner,
        ),
        event_data,
    );
}

// ── Review Archival Events (#804) ─────────────────────────────────────────────

/// Emitted for each review archived by `archive_old_reviews`.
///
/// Off-chain consumers (indexers, archival jobs) should subscribe to
/// `(REVIEW, ARCHIVED, project_id)` and persist the full review payload to
/// permanent storage (e.g., Arweave, IPFS) using the included fields.
/// The on-chain `ArchivedReview` record has a shorter TTL than active reviews,
/// so off-chain persistence is required for long-term retention.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewArchivedEvent {
    /// ID of the project the archived review belonged to.
    pub project_id: u64,
    /// Address of the reviewer whose review was archived.
    pub reviewer: Address,
    /// Rating of the archived review (1–5).
    pub rating: u32,
    /// Canonical content CID of the archived review (`None` if no off-chain content).
    pub content_cid: Option<soroban_sdk::String>,
    /// Unix timestamp (seconds) when the original review was submitted.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the review was last modified before archival.
    pub updated_at: u64,
    /// Unix timestamp (seconds) when the review was archived.
    pub archived_at: u64,
}

pub fn publish_review_archived_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    rating: u32,
    content_cid: Option<soroban_sdk::String>,
    created_at: u64,
    updated_at: u64,
) {
    let archived_at = env.ledger().timestamp();
    let event_data = ReviewArchivedEvent {
        project_id,
        reviewer: reviewer.clone(),
        rating,
        content_cid,
        created_at,
        updated_at,
        archived_at,
    };
    env.events().publish(
        (
            symbol_short!("REVIEW"),
            symbol_short!("ARCHIVED"),
            project_id,
            reviewer,
        ),
        event_data,
    );
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbationStartedEvent {
    pub project_id: u64,
    pub request_id: u64,
    pub approved_by: Address,
    pub started_at: u64,
    pub probation_until: u64,
}

pub fn publish_probation_started_event(
    env: &Env,
    project_id: u64,
    request_id: u64,
    approved_by: Address,
    started_at: u64,
    probation_until: u64,
) {
    let event = ProbationStartedEvent {
        project_id,
        request_id,
        approved_by,
        started_at,
        probation_until,
    };
    env.events().publish(
        (
            soroban_sdk::symbol_short!("PROBATION"),
            soroban_sdk::symbol_short!("STARTED"),
            project_id,
        ),
        event,
    );
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbationAutoPromotedEvent {
    pub project_id: u64,
    pub promoted_at: u64,
}

pub fn publish_probation_auto_promoted_event(env: &Env, project_id: u64, promoted_at: u64) {
    let event = ProbationAutoPromotedEvent {
        project_id,
        promoted_at,
    };
    env.events().publish(
        (
            soroban_sdk::symbol_short!("PROBATION"),
            soroban_sdk::symbol_short!("PROMOTED"),
            project_id,
        ),
        event,
    );
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbationRevokedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub reason: soroban_sdk::String,
    pub revoked_at: u64,
}

pub fn publish_probation_revoked_event(
    env: &Env,
    project_id: u64,
    admin: Address,
    reason: soroban_sdk::String,
    revoked_at: u64,
) {
    let event = ProbationRevokedEvent {
        project_id,
        admin,
        reason,
        revoked_at,
    };
    env.events().publish(
        (
            soroban_sdk::symbol_short!("PROBATION"),
            soroban_sdk::symbol_short!("REVOKED"),
            project_id,
        ),
        event,
    );
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbationIncidentEvent {
    pub project_id: u64,
    pub incident_id: u32,
    pub reporter: Address,
    pub details: soroban_sdk::String,
    pub recorded_at: u64,
}

pub fn publish_probation_incident_event(
    env: &Env,
    project_id: u64,
    incident_id: u32,
    reporter: Address,
    details: soroban_sdk::String,
    recorded_at: u64,
) {
    let event = ProbationIncidentEvent {
        project_id,
        incident_id,
        reporter,
        details,
        recorded_at,
    };
    env.events().publish(
        (
            soroban_sdk::symbol_short!("PROBATION"),
            soroban_sdk::symbol_short!("INCIDENT"),
            project_id,
        ),
        event,
    );
}
