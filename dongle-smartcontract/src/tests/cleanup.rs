//! Tests for stale-record cleanup functions:
//! - admin_delete_review
//! - clear_project_reports
//! - clear_verification_history
//! - clear_renewal_history
//! - admin reactivate_project
//! - index consistency after cleanup

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, String};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn valid_cid(env: &Env) -> String {
    String::from_str(
        env,
        "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
    )
}

fn setup(env: &Env) -> (crate::DongleContractClient<'_>, Address) {
    setup_contract(env)
}

// ═══════════════════════════════════════════════════════════════════════════
// admin_delete_review
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_admin_delete_review_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectA");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    // Sanity: review exists, stats reflect it
    let stats_before = client.get_project_stats(&project_id);
    assert_eq!(stats_before.review_count, 1);

    client.admin_delete_review(&project_id, &reviewer, &admin);

    // Review is gone
    assert!(client.get_review(&project_id, &reviewer).is_none());

    // Stats recalculated — review_count back to 0
    let stats_after = client.get_project_stats(&project_id);
    assert_eq!(stats_after.review_count, 0);
    assert_eq!(stats_after.rating_sum, 0);
}

#[test]
fn test_admin_delete_review_updates_project_reviews_index() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectB");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.add_review(&project_id, &r1, &5, &None);
    client.add_review(&project_id, &r2, &4, &None);

    client.admin_delete_review(&project_id, &r1, &admin);

    // r1 gone, r2 still present
    assert!(client.get_review(&project_id, &r1).is_none());
    assert!(client.get_review(&project_id, &r2).is_some());

    // list_reviews only returns r2
    let reviews = client.list_reviews(&project_id, &0, &100);
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews.get(0).unwrap().reviewer, r2);
}

#[test]
fn test_admin_delete_review_updates_user_reviews_index() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    // Two projects; reviewer reviewed both
    let pid1 = create_test_project(&client, &admin, "ProjectC");
    let pid2 = create_test_project(&client, &admin, "ProjectD");

    let reviewer = Address::generate(&env);
    client.add_review(&pid1, &reviewer, &5, &None);
    client.add_review(&pid2, &reviewer, &3, &None);

    // Admin deletes review on project 1 only
    client.admin_delete_review(&pid1, &reviewer, &admin);

    // Review on project 1 gone; project 2 review still intact
    assert!(client.get_review(&pid1, &reviewer).is_none());
    let r2 = client.get_review(&pid2, &reviewer);
    assert!(r2.is_some());
    assert_eq!(r2.unwrap().rating, 3);
}

#[test]
fn test_admin_delete_hidden_review_keeps_stats_consistent() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectE");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    // Hide the review first (stats already decremented)
    client.hide_review(&project_id, &reviewer, &admin);
    let stats_hidden = client.get_project_stats(&project_id);
    assert_eq!(stats_hidden.review_count, 0);

    // Admin delete — must not double-decrement
    client.admin_delete_review(&project_id, &reviewer, &admin);
    let stats_after = client.get_project_stats(&project_id);
    assert_eq!(stats_after.review_count, 0);
    assert_eq!(stats_after.rating_sum, 0);
}

#[test]
fn test_admin_delete_review_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectF");

    let reviewer = Address::generate(&env);
    let non_admin = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    let result = client.try_admin_delete_review(&project_id, &reviewer, &non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
    // Review still exists
    assert!(client.get_review(&project_id, &reviewer).is_some());
}

#[test]
fn test_admin_delete_review_missing_review_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectG");

    let ghost_reviewer = Address::generate(&env);
    let result = client.try_admin_delete_review(&project_id, &ghost_reviewer, &admin);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound)));
}

#[test]
fn test_admin_delete_review_missing_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let reviewer = Address::generate(&env);
    let result = client.try_admin_delete_review(&999u64, &reviewer, &admin);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

// ═══════════════════════════════════════════════════════════════════════════
// clear_project_reports
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_clear_project_reports_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectH");

    let reporter1 = Address::generate(&env);
    let reporter2 = Address::generate(&env);
    client.report_project(&project_id, &reporter1, &valid_cid(&env));
    client.report_project(&project_id, &reporter2, &valid_cid(&env));

    assert_eq!(client.get_project_report_count(&project_id), 2);

    client.clear_project_reports(&project_id, &admin);

    // Reports cleared
    assert_eq!(client.get_project_report_count(&project_id), 0);
    let reports = client.get_project_reports(&project_id);
    assert_eq!(reports.len(), 0);
}

#[test]
fn test_clear_project_reports_allows_re_reporting() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectI");

    let reporter = Address::generate(&env);
    client.report_project(&project_id, &reporter, &valid_cid(&env));

    // Cannot report twice before clearing
    let dup = client.try_report_project(&project_id, &reporter, &valid_cid(&env));
    assert_eq!(dup, Err(Ok(ContractError::AlreadyReported)));

    // Clear reports
    client.clear_project_reports(&project_id, &admin);

    // Now the same reporter can report again
    client.report_project(&project_id, &reporter, &valid_cid(&env));
    assert_eq!(client.get_project_report_count(&project_id), 1);
}

#[test]
fn test_clear_project_reports_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectJ");

    let reporter = Address::generate(&env);
    let non_admin = Address::generate(&env);
    client.report_project(&project_id, &reporter, &valid_cid(&env));

    let result = client.try_clear_project_reports(&project_id, &non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
    // Reports still there
    assert_eq!(client.get_project_report_count(&project_id), 1);
}

#[test]
fn test_clear_project_reports_no_reports_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectK");

    // No reports — should return the appropriate error
    let result = client.try_clear_project_reports(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_clear_project_reports_missing_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let result = client.try_clear_project_reports(&999u64, &admin);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_clear_project_reports_idempotent_second_clear_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectL");

    let reporter = Address::generate(&env);
    client.report_project(&project_id, &reporter, &valid_cid(&env));
    client.clear_project_reports(&project_id, &admin);

    // Second clear on now-empty project returns error
    let result = client.try_clear_project_reports(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

// ═══════════════════════════════════════════════════════════════════════════
// clear_verification_history
// ═══════════════════════════════════════════════════════════════════════════

/// Helper: get a project past the minimum age gate and request verification.
fn request_verification_for_project(
    env: &Env,
    client: &crate::DongleContractClient<'_>,
    project_id: u64,
    owner: &Address,
) {
    // Push ledger past the default minimum project age (0 in tests unless set)
    env.ledger().with_mut(|l| {
        l.timestamp = l.timestamp.saturating_add(1);
    });
    client.request_verification(&project_id, owner, &valid_cid(env));
}

#[test]
fn test_clear_verification_history_keep_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    // Disable minimum project age so verification can be requested immediately
    client.set_min_project_age(&admin, &0u64);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectM");

    // Request verification, reject, then request again — builds history
    request_verification_for_project(&env, &client, project_id, &owner);
    client.reject_verification(&project_id, &admin);
    request_verification_for_project(&env, &client, project_id, &owner);

    let history_before = client.get_verification_history(&project_id);
    assert_eq!(history_before.len(), 2);

    // Clear all history (keep_count = 0)
    let removed = client.clear_verification_history(&project_id, &admin, &0u32);
    assert_eq!(removed, 2);

    let history_after = client.get_verification_history(&project_id);
    assert_eq!(history_after.len(), 0);
}

#[test]
fn test_clear_verification_history_keep_one() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    client.set_min_project_age(&admin, &0u64);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectN");

    request_verification_for_project(&env, &client, project_id, &owner);
    client.reject_verification(&project_id, &admin);
    request_verification_for_project(&env, &client, project_id, &owner);
    client.reject_verification(&project_id, &admin);
    request_verification_for_project(&env, &client, project_id, &owner);

    let history_before = client.get_verification_history(&project_id);
    assert_eq!(history_before.len(), 3);

    // Keep only the most recent 1
    let removed = client.clear_verification_history(&project_id, &admin, &1u32);
    assert_eq!(removed, 2);

    let history_after = client.get_verification_history(&project_id);
    assert_eq!(history_after.len(), 1);
}

#[test]
fn test_clear_verification_history_keep_count_gte_total_removes_nothing() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    client.set_min_project_age(&admin, &0u64);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectO");

    request_verification_for_project(&env, &client, project_id, &owner);

    // keep_count=10 but only 1 record — nothing removed
    let removed = client.clear_verification_history(&project_id, &admin, &10u32);
    assert_eq!(removed, 0);

    let history = client.get_verification_history(&project_id);
    assert_eq!(history.len(), 1);
}

#[test]
fn test_clear_verification_history_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    client.set_min_project_age(&admin, &0u64);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectP");
    request_verification_for_project(&env, &client, project_id, &owner);

    let non_admin = Address::generate(&env);
    let result = client.try_clear_verification_history(&project_id, &non_admin, &0u32);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_clear_verification_history_missing_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let result = client.try_clear_verification_history(&999u64, &admin, &0u32);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_clear_verification_history_empty_returns_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectQ");

    // No verification requests at all
    let removed = client.clear_verification_history(&project_id, &admin, &0u32);
    assert_eq!(removed, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// clear_renewal_history
// ═══════════════════════════════════════════════════════════════════════════

fn verify_and_approve(
    env: &Env,
    client: &crate::DongleContractClient<'_>,
    project_id: u64,
    owner: &Address,
    admin: &Address,
) {
    env.ledger().with_mut(|l| {
        l.timestamp = l.timestamp.saturating_add(1);
    });
    client.request_verification(&project_id, owner, &valid_cid(env));
    client.approve_verification(&project_id, admin);
}

fn do_renewal(
    env: &Env,
    client: &crate::DongleContractClient<'_>,
    project_id: u64,
    owner: &Address,
    admin: &Address,
) {
    env.ledger().with_mut(|l| {
        l.timestamp = l.timestamp.saturating_add(1);
    });
    client.request_renewal(&project_id, owner, &valid_cid(env));
    client.approve_renewal(&project_id, admin);
}

#[test]
fn test_clear_renewal_history_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    client.set_min_project_age(&admin, &0u64);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectR");

    verify_and_approve(&env, &client, project_id, &owner, &admin);
    do_renewal(&env, &client, project_id, &owner, &admin);
    do_renewal(&env, &client, project_id, &owner, &admin);

    let history_before = client.get_renewal_history(&project_id, &0u32, &100u32);
    assert_eq!(history_before.len(), 2);

    let removed = client.clear_renewal_history(&project_id, &admin);
    assert_eq!(removed, 2);

    let history_after = client.get_renewal_history(&project_id, &0u32, &100u32);
    assert_eq!(history_after.len(), 0);
}

#[test]
fn test_clear_renewal_history_empty_returns_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    client.set_min_project_age(&admin, &0u64);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectS");

    verify_and_approve(&env, &client, project_id, &owner, &admin);
    // No renewals submitted — clear should return 0
    let removed = client.clear_renewal_history(&project_id, &admin);
    assert_eq!(removed, 0);
}

#[test]
fn test_clear_renewal_history_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    client.set_min_project_age(&admin, &0u64);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectT");
    verify_and_approve(&env, &client, project_id, &owner, &admin);
    do_renewal(&env, &client, project_id, &owner, &admin);

    let non_admin = Address::generate(&env);
    let result = client.try_clear_renewal_history(&project_id, &non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));

    // History untouched
    let history = client.get_renewal_history(&project_id, &0u32, &100u32);
    assert_eq!(history.len(), 1);
}

#[test]
fn test_clear_renewal_history_missing_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let result = client.try_clear_renewal_history(&999u64, &admin);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

// ═══════════════════════════════════════════════════════════════════════════
// admin reactivate_project
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_admin_can_reactivate_archived_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectU");

    client.archive_project(&project_id, &owner);
    assert!(client.get_project(&project_id).unwrap().archived);

    // Admin reactivates (not the owner)
    client.reactivate_project(&project_id, &admin);
    assert!(!client.get_project(&project_id).unwrap().archived);
}

#[test]
fn test_archive_already_archived_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectV");
    client.archive_project(&project_id, &owner);

    let result = client.try_archive_project(&project_id, &owner);
    assert_eq!(result, Err(Ok(ContractError::AlreadyArchived)));
}

#[test]
fn test_reactivate_not_archived_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectW");

    let result = client.try_reactivate_project(&project_id, &owner);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotArchived)));
}

#[test]
fn test_stranger_cannot_archive_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectX");

    let result = client.try_archive_project(&project_id, &stranger);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

// ═══════════════════════════════════════════════════════════════════════════
// index consistency after cleanup
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_admin_delete_review_stats_multiple_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectY");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    client.add_review(&project_id, &r1, &4, &None);
    client.add_review(&project_id, &r2, &2, &None);
    client.add_review(&project_id, &r3, &5, &None);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 3);
    // ratings stored as x*100: (400+200+500) = 1100
    assert_eq!(stats.rating_sum, 1100);

    // Admin removes the rating-2 review
    client.admin_delete_review(&project_id, &r2, &admin);
    let stats2 = client.get_project_stats(&project_id);
    assert_eq!(stats2.review_count, 2);
    assert_eq!(stats2.rating_sum, 900); // 400+500

    // Remaining reviews still accessible
    assert!(client.get_review(&project_id, &r1).is_some());
    assert!(client.get_review(&project_id, &r2).is_none());
    assert!(client.get_review(&project_id, &r3).is_some());
}

#[test]
fn test_clear_reports_then_report_updates_count_correctly() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectZ");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.report_project(&project_id, &r1, &valid_cid(&env));
    client.report_project(&project_id, &r2, &valid_cid(&env));
    assert_eq!(client.get_project_report_count(&project_id), 2);

    client.clear_project_reports(&project_id, &admin);
    assert_eq!(client.get_project_report_count(&project_id), 0);

    // Fresh reports after clearing
    let r3 = Address::generate(&env);
    client.report_project(&project_id, &r3, &valid_cid(&env));
    assert_eq!(client.get_project_report_count(&project_id), 1);
}

#[test]
fn test_archived_project_still_accessible_via_get_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectAA");
    client.archive_project(&project_id, &owner);

    let project = client.get_project(&project_id);
    assert!(project.is_some());
    assert!(project.unwrap().archived);
}

#[test]
fn test_archived_project_excluded_from_list_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let id1 = create_test_project(&client, &owner, "ProjectBB");
    let id2 = create_test_project(&client, &owner, "ProjectCC");

    client.archive_project(&id1, &owner);

    let projects = client.list_projects(&1u64, &100u32);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, id2);
}
