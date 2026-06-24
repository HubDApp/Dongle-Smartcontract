use crate::types::{AdminActionType, ReviewAction, ReviewEventData};
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
        (
            symbol_short!("VERIFY"),
            symbol_short!("REVOKED"),
            project_id,
        ),
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
pub struct ProjectRemovedFromCollectionEvent {
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
    let event_data = ProjectRemovedFromCollectionEvent {
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

pub fn publish_project_linked_event(
    env: &Env,
    project_id: u64,
    linked_project_id: u64,
    owner: Address,
) {
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("LINKED"),
            project_id,
        ),
        (linked_project_id, owner, env.ledger().timestamp()),
    );
}

pub fn publish_project_unlinked_event(
    env: &Env,
    project_id: u64,
    linked_project_id: u64,
    owner: Address,
) {
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("UNLINKED"),
            project_id,
        ),
        (linked_project_id, owner, env.ledger().timestamp()),
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
