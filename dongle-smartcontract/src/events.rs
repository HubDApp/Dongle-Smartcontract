use crate::types::{ReviewAction, ReviewEventData};
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

pub const REVIEW: Symbol = symbol_short!("REVIEW");

// ── Standardized event data structs ──────────────────────────────────────────

/// Emitted when a project is registered.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegisteredEvent {
    pub project_id: u64,
    pub owner: Address,
    pub name: String,
    pub category: String,
    pub timestamp: u64,
}

/// Emitted when a project is updated.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

/// Emitted when a verification is requested.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRequestedEvent {
    pub project_id: u64,
    pub requester: Address,
    pub evidence_cid: String,
    pub timestamp: u64,
}

/// Emitted when a verification is approved.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationApprovedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

/// Emitted when a verification is rejected.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRejectedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

/// Emitted when a verification is revoked by an admin.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRevokedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub reason: String,
    pub timestamp: u64,
}

/// Emitted when project ownership is transferred.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectOwnershipTransferredEvent {
    pub project_id: u64,
    pub old_owner: Address,
    pub new_owner: Address,
    pub timestamp: u64,
}

/// Emitted when a project is archived.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectArchivedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

/// Emitted when a project is reactivated.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReactivatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}

/// Emitted when an admin is added.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAddedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

/// Emitted when an admin is removed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRemovedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

/// Emitted when fees are configured.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeSetEvent {
    pub verification_fee: u128,
    pub registration_fee: u128,
    pub timestamp: u64,
}

/// Emitted when a fee is paid for a project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaidEvent {
    pub project_id: u64,
    pub payer: Address,
    pub amount: u128,
    pub timestamp: u64,
}

// ── Review events ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn publish_review_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    action: ReviewAction,
    ipfs_cid: Option<String>,
    comment_cid: Option<String>,
    owner_response: Option<String>,
    created_at: u64,
    updated_at: u64,
) {
    let event_data = ReviewEventData {
        project_id,
        reviewer: reviewer.clone(),
        action: action.clone(),
        timestamp: env.ledger().timestamp(),
        ipfs_cid,
        created_at,
        updated_at,
        comment_cid,
        owner_response,
    };

    let action_sym = match action {
        ReviewAction::Submitted => symbol_short!("SUBMITTED"),
        ReviewAction::Updated => symbol_short!("UPDATED"),
        ReviewAction::Deleted => symbol_short!("DELETED"),
    };

    env.events()
        .publish((REVIEW, action_sym, project_id, reviewer), event_data);
}

// ── Project events ────────────────────────────────────────────────────────────

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

// ── Fee events ────────────────────────────────────────────────────────────────

pub fn publish_fee_paid_event(env: &Env, project_id: u64, payer: Address, amount: u128) {
    let event_data = FeePaidEvent {
        project_id,
        payer,
        amount,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("PAID"), project_id),
        event_data,
    );
}

pub fn publish_fee_set_event(env: &Env, verification_fee: u128, registration_fee: u128) {
    let event_data = FeeSetEvent {
        verification_fee,
        registration_fee,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("FEE"), symbol_short!("SET")), event_data);
}

// ── Verification events ───────────────────────────────────────────────────────

pub fn publish_verification_requested_event(
    env: &Env,
    project_id: u64,
    requester: Address,
    evidence_cid: String,
) {
    let event_data = VerificationRequestedEvent {
        project_id,
        requester,
        evidence_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REQ"), project_id),
        event_data,
    );
}

pub fn publish_verification_approved_event(env: &Env, project_id: u64, admin: Address) {
    let event_data = VerificationApprovedEvent {
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("APP"), project_id),
        event_data,
    );
}

pub fn publish_verification_rejected_event(env: &Env, project_id: u64, admin: Address) {
    let event_data = VerificationRejectedEvent {
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REJ"), project_id),
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
        (symbol_short!("VERIFY"), symbol_short!("REVOKED"), project_id),
        event_data,
    );
}

// ── Admin events ──────────────────────────────────────────────────────────────

pub fn publish_ownership_transferred_event(
    env: &Env,
    project_id: u64,
    old_owner: Address,
    new_owner: Address,
) {
    let event_data = ProjectOwnershipTransferredEvent {
        project_id,
        old_owner,
        new_owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("PROJECT"), symbol_short!("TRANSFER"), project_id),
        event_data,
    );
}

// ── Admin events ──────────────────────────────────────────────────────────────

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

/// Emitted when a review is reported.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewReportedEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub reporter: Address,
    pub timestamp: u64,
}

/// Emitted when a review is hidden by moderation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewHiddenEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub admin: Address,
    pub timestamp: u64,
}

/// Emitted when a review is restored by moderation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewRestoredEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_review_reported_event(env: &Env, project_id: u64, reviewer: Address, reporter: Address) {
    let event_data = ReviewReportedEvent {
        project_id,
        reviewer,
        reporter,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("REVIEW"), symbol_short!("REPORTED"), project_id),
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

pub fn publish_review_restored_event(env: &Env, project_id: u64, reviewer: Address, admin: Address) {
    let event_data = ReviewRestoredEvent {
        project_id,
        reviewer,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("REVIEW"), symbol_short!("RESTORED"), project_id),
        event_data,
    );
}

pub fn publish_project_archived_event(env: &Env, project_id: u64, owner: Address) {
    let event_data = ProjectArchivedEvent {
        project_id,
        owner,
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

pub fn publish_project_reactivated_event(env: &Env, project_id: u64, owner: Address) {
    let event_data = ProjectReactivatedEvent {
        project_id,
        owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("REACTIVED"),
            project_id,
        ),
        event_data,
    );
}

/// Emitted when a verification renewal is requested.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewalRequestedEvent {
    pub project_id: u64,
    pub requester: Address,
    pub evidence_cid: String,
    pub timestamp: u64,
}

/// Emitted when a verification renewal is approved.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewalApprovedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub expires_at: u64,
    pub timestamp: u64,
}

/// Emitted when a verification renewal is rejected.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRenewalRejectedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

pub fn publish_verification_renewal_requested_event(
    env: &Env,
    project_id: u64,
    requester: Address,
    evidence_cid: String,
) {
    let event_data = VerificationRenewalRequestedEvent {
        project_id,
        requester,
        evidence_cid,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("RENEW_REQ"), project_id),
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
        (symbol_short!("VERIFY"), symbol_short!("RENEW_APP"), project_id),
        event_data,
    );
}

pub fn publish_verification_renewal_rejected_event(
    env: &Env,
    project_id: u64,
    admin: Address,
) {
    let event_data = VerificationRenewalRejectedEvent {
        project_id,
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("RENEW_REJ"), project_id),
        event_data,
    );
}
