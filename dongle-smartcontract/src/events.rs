use crate::types::{ReviewAction, ReviewEventData};
use soroban_sdk::{symbol_short, Address, Env, String, Symbol};

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
    created_at: u64,
    updated_at: u64,
) {
    let event_data = ReviewEventData {
        project_id,
        reviewer: reviewer.clone(),
        action: action.clone(),
        created_at,
        updated_at,
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn publish_verification_requested_event(
    env: &Env,
    project_id: u64,
    requester: Address,
    evidence_cid: String,
) {
     env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REQ"), project_id),
        (requester, evidence_cid),
    );
}

pub fn publish_verification_approved_event(env: &Env, project_id: u64, admin: Address) {
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("APP"), project_id),
        admin,
    );
}

pub fn publish_verification_rejected_event(env: &Env, project_id: u64, admin: Address) {
    env.events().publish(
        (symbol_short!("VERIFY"), symbol_short!("REJ"), project_id),
        admin,
    );
}

pub fn publish_admin_added_event(env: &Env, admin: Address) {
    env.events()
        .publish((symbol_short!("ADMIN"), symbol_short!("ADDED")), admin);
}

pub fn publish_admin_removed_event(env: &Env, admin: Address) {
    env.events()
        .publish((symbol_short!("ADMIN"), symbol_short!("REMOVED")), admin);
}
