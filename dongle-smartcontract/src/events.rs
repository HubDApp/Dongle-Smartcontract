use crate::types::{ReviewAction, ReviewEventData};
use soroban_sdk::{Address, Env, String, Symbol, symbol_short};

pub const REVIEW: Symbol = symbol_short!("REVIEW");
pub const FEE_COL: Symbol = symbol_short!("FEE_COL");
pub const TREAS_WD: Symbol = symbol_short!("TREAS_WD");
pub const VER_REQ: Symbol = symbol_short!("VER_REQ");
pub const VER_APP: Symbol = symbol_short!("VER_APP");
pub const VER_REJ: Symbol = symbol_short!("VER_REJ");

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

pub fn publish_fee_collected_event(
    env: &Env,
    payer: Address,
    project_id: u64,
    token: Address,
    amount: u128,
) {
    env.events()
        .publish((FEE_COL, payer, project_id), (token, amount));
}

pub fn publish_treasury_withdrawal_event(
    env: &Env,
    token: Address,
    amount: u128,
    to: Address,
) {
    env.events()
        .publish((TREAS_WD, to), (token, amount));
}

pub fn publish_verification_requested_event(env: &Env, project_id: u64, requester: Address) {
    env.events()
        .publish((VER_REQ, requester, project_id), ());
}

pub fn publish_verification_approved_event(env: &Env, project_id: u64) {
    env.events()
        .publish((VER_APP, project_id), ());
}

pub fn publish_verification_rejected_event(env: &Env, project_id: u64) {
    env.events()
        .publish((VER_REJ, project_id), ());
}
