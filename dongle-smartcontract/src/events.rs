use crate::types::{ReviewAction, ReviewEventData, VerificationStatus};
use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec};

pub const REVIEW: Symbol = symbol_short!("REVIEW");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperation {
    Verification,
    Registration,
}

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
pub struct VerificationRequestedEvent {
    pub project_id: u64,
    pub requester: Address,
    pub evidence_cid: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationApprovedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRejectedEvent {
    pub project_id: u64,
    pub admin: Address,
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
pub struct VerificationRenewalRequestedEvent {
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
pub struct ProjectOwnershipTransferredEvent {
    pub project_id: u64,
    pub caller: Address,
    pub old_owner: Address,
    pub new_owner: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectArchivedEvent {
    pub project_id: u64,
    pub archived_by: Address,
    pub owner: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReactivatedEvent {
    pub project_id: u64,
    pub caller: Address,
    pub archived: bool,
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
pub struct ProjectReportedEvent {
    pub project_id: u64,
    pub reporter: Address,
    pub reason_cid: String,
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
pub struct MinProjectAgeSetEvent {
    pub admin: Address,
    pub previous_min_age_seconds: u64,
    pub min_age_seconds: u64,
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
pub struct ReviewReportedEvent {
    pub project_id: u64,
    pub caller: Address,
    pub reviewer: Address,
    pub report_count: u32,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewHiddenEvent {
    pub project_id: u64,
    pub admin: Address,
    pub reviewer: Address,
    pub hidden: bool,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewRestoredEvent {
    pub project_id: u64,
    pub admin: Address,
    pub reviewer: Address,
    pub hidden: bool,
    pub timestamp: u64,
}

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
    };

    let action_sym = match action {
        ReviewAction::Submitted => symbol_short!("SUBMITTED"),
        ReviewAction::Updated => symbol_short!("UPDATED"),
        ReviewAction::Deleted => symbol_short!("DELETED"),
    };

    env.events()
        .publish((REVIEW, action_sym, project_id, reviewer), event_data);
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

// ── Fee events ────────────────────────────────────────────────────────────────

pub fn publish_project_reactivated_event(env: &Env, project_id: u64, caller: Address) {
    let event_data = ProjectReactivatedEvent {
        project_id,
        caller,
        archived: false,
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

pub fn publish_verification_renewal_requested_event(
    env: &Env,
    project_id: u64,
    requester: Address,
    evidence_cid: String,
    fee_amount: u128,
) {
    let event_data = VerificationRenewalRequestedEvent {
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
        (symbol_short!("RENEW"), symbol_short!("APPROVED"), project_id),
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
        (symbol_short!("RENEW"), symbol_short!("REJECTED"), project_id),
        event_data,
    );
}

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
        (symbol_short!("PROJECT"), symbol_short!("TRANSFER"), project_id),
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
        (symbol_short!("PROJECT"), symbol_short!("REPORTED"), project_id),
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
        (symbol_short!("PROJECT"), symbol_short!("SOCIAL"), project_id),
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

pub fn publish_review_reported_event(env: &Env, project_id: u64, reviewer: Address, reporter: Address) {
    let event_data = ReviewReportedEvent {
        project_id,
        reviewer,
        reporter,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (symbol_short!(\"REVIEW\"), symbol_short!(\"REPORTED\"), project_id),
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
        (symbol_short!(\"REVIEW\"), symbol_short!(\"HIDDEN\"), project_id),
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
        (symbol_short!(\"REVIEW\"), symbol_short!(\"RESTORED\"), project_id),
        event_data,
    );
}
