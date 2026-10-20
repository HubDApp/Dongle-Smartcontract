//! Batch review moderation tests: hide/delete/flag/restore, partial failure,
//! audit trail, and per-admin rate limiting.

use crate::constants::{
    MAX_MODERATION_BATCH_SIZE, MODERATION_ACTIONS_PER_WINDOW, MODERATION_RATE_LIMIT_WINDOW_SECONDS,
};
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{AdminActionType, ModerateAction};
use crate::DongleContractClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env, Vec,
};

/// Create a project with `count` reviews (rating 4 each) and return the
/// `(project_id, reviewer)` pairs.
fn setup_reviews(
    env: &Env,
    client: &DongleContractClient<'_>,
    admin: &Address,
    name: &str,
    count: u32,
) -> Vec<(u64, Address)> {
    let project_id = create_test_project(client, admin, name);
    let mut ids = Vec::new(env);
    for _ in 0..count {
        let reviewer = Address::generate(env);
        client.add_review(&project_id, &reviewer, &4, &None);
        ids.push_back((project_id, reviewer));
    }
    ids
}

#[test]
fn test_batch_hide_hides_all_and_updates_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let ids = setup_reviews(&env, &client, &admin, "BatchHide", 3);
    let project_id = ids.get(0).unwrap().0;

    let result = client.moderate_reviews_batch(&admin, &ids, &ModerateAction::Hide);

    assert_eq!(result.action, ModerateAction::Hide);
    assert_eq!(result.successful, ids);
    assert!(result.failed.is_empty());
    for (pid, reviewer) in ids.iter() {
        assert!(client.get_review(&pid, &reviewer).unwrap().hidden);
    }
    assert_eq!(client.get_project_stats(&project_id).review_count, 0);
}

#[test]
fn test_batch_partial_failure_keeps_successes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let ids = setup_reviews(&env, &client, &admin, "BatchPartial", 2);
    let (project_id, first) = ids.get(0).unwrap();
    let (_, second) = ids.get(1).unwrap();
    let missing = Address::generate(&env);

    // First review is already hidden, so hiding it again must fail.
    client.hide_review(&project_id, &first, &admin);

    let batch = vec![
        &env,
        (project_id, first.clone()),
        (project_id, missing.clone()),
        (project_id, second.clone()),
        (999u64, second.clone()),
    ];
    let result = client.moderate_reviews_batch(&admin, &batch, &ModerateAction::Hide);

    assert_eq!(result.successful, vec![&env, (project_id, second.clone())]);
    assert_eq!(result.failed.len(), 3);
    let codes: std::vec::Vec<u32> = result.failed.iter().map(|f| f.error_code).collect();
    assert_eq!(
        codes,
        [
            ContractError::ReviewAlreadyHidden as u32,
            ContractError::ReviewNotFound as u32,
            ContractError::ProjectNotFound as u32,
        ]
    );
    assert_eq!(result.failed.get(1).unwrap().reviewer, missing);
    assert!(client.get_review(&project_id, &second).unwrap().hidden);
}

#[test]
fn test_batch_delete_removes_reviews_and_flags() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let ids = setup_reviews(&env, &client, &admin, "BatchDelete", 2);
    let project_id = ids.get(0).unwrap().0;

    client.moderate_reviews_batch(&admin, &ids, &ModerateAction::Flag);
    let result = client.moderate_reviews_batch(&admin, &ids, &ModerateAction::Delete);

    assert_eq!(result.successful.len(), 2);
    for (pid, reviewer) in ids.iter() {
        assert!(client.get_review(&pid, &reviewer).is_none());
        assert!(client.get_review_tombstone(&pid, &reviewer).is_some());
        assert!(client.get_review_flag(&pid, &reviewer).is_none());
    }
    assert_eq!(client.get_project_stats(&project_id).review_count, 0);
}

#[test]
fn test_batch_flag_marks_without_hiding() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 5_000);
    let (client, admin) = setup_contract(&env);
    let ids = setup_reviews(&env, &client, &admin, "BatchFlag", 2);
    let project_id = ids.get(0).unwrap().0;

    let result = client.moderate_reviews_batch(&admin, &ids, &ModerateAction::Flag);
    assert_eq!(result.successful.len(), 2);

    for (pid, reviewer) in ids.iter() {
        let flag = client.get_review_flag(&pid, &reviewer).unwrap();
        assert_eq!(flag.flagged_by, admin);
        assert_eq!(flag.flagged_at, 5_000);
        assert!(!client.get_review(&pid, &reviewer).unwrap().hidden);
    }
    assert_eq!(client.get_project_stats(&project_id).review_count, 2);

    // Flagging twice reports ReviewAlreadyFlagged per item.
    let again = client.moderate_reviews_batch(&admin, &ids, &ModerateAction::Flag);
    assert!(again.successful.is_empty());
    for failure in again.failed.iter() {
        assert_eq!(failure.error_code, ContractError::ReviewAlreadyFlagged as u32);
    }
}

#[test]
fn test_batch_restore_unhides_and_unflags() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let ids = setup_reviews(&env, &client, &admin, "BatchRestore", 3);
    let (project_id, hidden) = ids.get(0).unwrap();
    let (_, flagged) = ids.get(1).unwrap();
    let (_, untouched) = ids.get(2).unwrap();

    client.hide_review(&project_id, &hidden, &admin);
    client.moderate_reviews_batch(
        &admin,
        &vec![&env, (project_id, flagged.clone())],
        &ModerateAction::Flag,
    );

    let result = client.moderate_reviews_batch(&admin, &ids, &ModerateAction::Restore);

    assert_eq!(
        result.successful,
        vec![&env, (project_id, hidden.clone()), (project_id, flagged.clone())]
    );
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed.get(0).unwrap().reviewer, untouched);
    assert_eq!(
        result.failed.get(0).unwrap().error_code,
        ContractError::ReviewNotHidden as u32
    );
    assert!(!client.get_review(&project_id, &hidden).unwrap().hidden);
    assert!(client.get_review_flag(&project_id, &flagged).is_none());
    assert_eq!(client.get_project_stats(&project_id).review_count, 3);
}

#[test]
fn test_batch_records_audit_entry_per_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let ids = setup_reviews(&env, &client, &admin, "BatchAudit", 2);
    let project_id = ids.get(0).unwrap().0;

    let mut batch = ids.clone();
    batch.push_back((project_id, Address::generate(&env)));

    let before = client.get_admin_action_log_count();
    client.moderate_reviews_batch(&admin, &batch, &ModerateAction::Flag);
    let after = client.get_admin_action_log_count();

    // One entry per successful item; the failed item is not logged.
    assert_eq!(after - before, 2);
    let entries = client.list_admin_actions(&0, &2);
    for entry in entries.iter() {
        assert_eq!(entry.admin, admin);
        assert_eq!(entry.action_type, AdminActionType::ReviewFlagged);
        assert_eq!(entry.target_id, Some(project_id));
    }
}

#[test]
fn test_batch_requires_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let ids = setup_reviews(&env, &client, &admin, "BatchAuth", 1);
    let outsider = Address::generate(&env);

    let result = client.try_moderate_reviews_batch(&outsider, &ids, &ModerateAction::Hide);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly.into())));
}

#[test]
fn test_batch_rejects_empty_and_oversized() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let empty: Vec<(u64, Address)> = Vec::new(&env);
    assert_eq!(
        client.try_moderate_reviews_batch(&admin, &empty, &ModerateAction::Hide),
        Err(Ok(ContractError::InvalidInput.into()))
    );

    let mut oversized = Vec::new(&env);
    for _ in 0..=MAX_MODERATION_BATCH_SIZE {
        oversized.push_back((1u64, Address::generate(&env)));
    }
    assert_eq!(
        client.try_moderate_reviews_batch(&admin, &oversized, &ModerateAction::Hide),
        Err(Ok(ContractError::InvalidInput.into()))
    );
    // Rejected batches spend no budget.
    assert_eq!(
        client.get_moderation_budget(&admin),
        MODERATION_ACTIONS_PER_WINDOW
    );
}

#[test]
fn test_batch_rate_limit_and_window_reset() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 10_000);
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "BatchRate");

    // Failed items count toward the budget, so batches of unknown reviews
    // are enough to exhaust it.
    let full_batches = MODERATION_ACTIONS_PER_WINDOW / MAX_MODERATION_BATCH_SIZE;
    for _ in 0..full_batches {
        let mut batch = Vec::new(&env);
        for _ in 0..MAX_MODERATION_BATCH_SIZE {
            batch.push_back((project_id, Address::generate(&env)));
        }
        let result = client.moderate_reviews_batch(&admin, &batch, &ModerateAction::Flag);
        assert_eq!(result.failed.len(), MAX_MODERATION_BATCH_SIZE);
    }
    assert_eq!(client.get_moderation_budget(&admin), 0);

    let ids = setup_reviews(&env, &client, &admin, "BatchRate2", 1);
    assert_eq!(
        client.try_moderate_reviews_batch(&admin, &ids, &ModerateAction::Hide),
        Err(Ok(ContractError::ModerationRateLimitExceeded.into()))
    );
    let (pid, reviewer) = ids.get(0).unwrap();
    assert!(!client.get_review(&pid, &reviewer).unwrap().hidden);

    // Budget resets in the next window.
    env.ledger().with_mut(|li| {
        li.timestamp += MODERATION_RATE_LIMIT_WINDOW_SECONDS;
    });
    assert_eq!(
        client.get_moderation_budget(&admin),
        MODERATION_ACTIONS_PER_WINDOW
    );
    let result = client.moderate_reviews_batch(&admin, &ids, &ModerateAction::Hide);
    assert_eq!(result.successful.len(), 1);
}
