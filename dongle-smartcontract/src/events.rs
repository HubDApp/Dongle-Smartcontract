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
