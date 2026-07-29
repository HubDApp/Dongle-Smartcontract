use crate::types::{DisputeResolutionAction, VerificationStatus};
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

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
pub struct VerificationRevokedEvent {
    pub project_id: u64,
    pub admin: Address,
    pub reason: String,
    pub timestamp: u64,
}

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
pub struct VerificationAssignedEvent {
    pub project_id: u64,
    pub request_id: u64,
    pub assigned_admin: Address,
    pub assigner: Address,
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
    pub action: DisputeResolutionAction,
    pub timestamp: u64,
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
    action: DisputeResolutionAction,
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
