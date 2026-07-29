use crate::types::{ReviewAction, ReviewEventData, ReviewRevisionEvent};
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

pub const REVIEW: Symbol = symbol_short!("REVIEW");

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
        ReviewAction::Revised => symbol_short!("REVISED"),
        ReviewAction::Deleted => symbol_short!("DELETED"),
    };

    env.events()
        .publish((REVIEW, action_sym, project_id, reviewer), event_data);
}

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
