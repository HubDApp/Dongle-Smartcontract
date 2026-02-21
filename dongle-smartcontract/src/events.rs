use crate::types::{ReviewAction, ReviewEventData};
use soroban_sdk::{Address, Env, String};

pub const REVIEW: &str = "Review";

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

    let action_str = match action {
        ReviewAction::Submitted => "Submitted",
        ReviewAction::Updated => "Updated",
        ReviewAction::Deleted => "Deleted",
    };

    env.events()
        .publish((REVIEW, action_str, project_id, reviewer), event_data);
}
