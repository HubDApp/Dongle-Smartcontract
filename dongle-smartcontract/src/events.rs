use crate::types::{ReviewAction, ReviewEventData};
use soroban_sdk::{Address, Env, String, Symbol, symbol_short};

pub const REVIEW: Symbol = symbol_short!("REVIEW");

pub fn publish_review_event(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    action: ReviewAction,
    comment_cid: Option<String>,
) {
    let event_data = ReviewEventData {
        project_id,
        reviewer: reviewer.clone(),
        action: action.clone(),
        timestamp: env.ledger().timestamp(),
        comment_cid,
    };

    let action_sym = match action {
        ReviewAction::Submitted => symbol_short!("SUBMITTED"),
        ReviewAction::Updated => symbol_short!("UPDATED"),
        ReviewAction::Deleted => symbol_short!("DELETED"),
    };

    env.events()
        .publish((REVIEW, action_sym, project_id, reviewer), event_data);
}

pub fn publish_fee_paid_event(env: &Env, project_id: u64, amount: u128) {
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("PAID"), project_id),
        amount,
    );
}

pub fn publish_fee_set_event(env: &Env, verification_fee: u128, registration_fee: u128) {
    env.events().publish(
        (symbol_short!("FEE"), symbol_short!("SET")),
        (verification_fee, registration_fee),
    );
}

pub fn publish_verification_requested_event(env: &Env, project_id: u64, requester: Address) {
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REQ"), project_id),
        requester,
    );
}

pub fn publish_verification_approved_event(env: &Env, project_id: u64) {
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("APP"), project_id),
        project_id,
    );
}

pub fn publish_verification_rejected_event(env: &Env, project_id: u64) {
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REJ"), project_id),
        project_id,
    );
}
