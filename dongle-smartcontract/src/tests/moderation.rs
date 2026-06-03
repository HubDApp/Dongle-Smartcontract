//! Review moderation tests: reporting, hiding, restoring, and stats behavior.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    setup_contract(env)
}

// ---------------------------------------------------------------------------
// report_review
// ---------------------------------------------------------------------------

#[test]
fn test_report_review_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectA");

    let reviewer = Address::generate(&env);
    let reporter = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    client.report_review(&project_id, &reviewer, &reporter);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.report_count, 1);
}

#[test]
fn test_report_review_multiple_reporters() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectB");

    let reviewer = Address::generate(&env);
    let reporter1 = Address::generate(&env);
    let reporter2 = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    client.report_review(&project_id, &reviewer, &reporter1);
    client.report_review(&project_id, &reviewer, &reporter2);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.report_count, 2);
}

#[test]
fn test_report_review_duplicate_reporter_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectC");

    let reviewer = Address::generate(&env);
    let reporter = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    client.report_review(&project_id, &reviewer, &reporter);

    let result = client.try_report_review(&project_id, &reviewer, &reporter);
    assert_eq!(result, Err(Ok(ContractError::ReviewAlreadyReported.into())));
}

#[test]
fn test_report_review_nonexistent_review_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectD");

    let reviewer = Address::generate(&env);
    let reporter = Address::generate(&env);

    let result = client.try_report_review(&project_id, &reviewer, &reporter);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound.into())));
}

#[test]
fn test_report_review_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let reviewer = Address::generate(&env);
    let reporter = Address::generate(&env);

    let result = client.try_report_review(&999, &reviewer, &reporter);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound.into())));
}

// ---------------------------------------------------------------------------
// hide_review
// ---------------------------------------------------------------------------

#[test]
fn test_hide_review_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectE");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    client.hide_review(&project_id, &reviewer, &admin);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.hidden, true);
}

#[test]
fn test_hide_review_updates_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectF");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    let stats_before = client.get_project_stats(&project_id);
    assert_eq!(stats_before.review_count, 1);
    assert_eq!(stats_before.rating_sum, 500);

    client.hide_review(&project_id, &reviewer, &admin);

    let stats_after = client.get_project_stats(&project_id);
    assert_eq!(stats_after.review_count, 0);
    assert_eq!(stats_after.rating_sum, 0);
    assert_eq!(stats_after.average_rating, 0);
}

#[test]
fn test_hide_review_already_hidden_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectG");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);
    client.hide_review(&project_id, &reviewer, &admin);

    let result = client.try_hide_review(&project_id, &reviewer, &admin);
    assert_eq!(result, Err(Ok(ContractError::ReviewAlreadyHidden.into())));
}

#[test]
fn test_hide_review_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectH");

    let reviewer = Address::generate(&env);
    let non_admin = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    let result = client.try_hide_review(&project_id, &reviewer, &non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly.into())));
}

#[test]
fn test_hide_review_nonexistent_review_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectI");

    let reviewer = Address::generate(&env);

    let result = client.try_hide_review(&project_id, &reviewer, &admin);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound.into())));
}

#[test]
fn test_hide_review_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let reviewer = Address::generate(&env);

    let result = client.try_hide_review(&999, &reviewer, &admin);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound.into())));
}

// ---------------------------------------------------------------------------
// restore_review
// ---------------------------------------------------------------------------

#[test]
fn test_restore_review_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectJ");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);
    client.hide_review(&project_id, &reviewer, &admin);

    client.restore_review(&project_id, &reviewer, &admin);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.hidden, false);
}

#[test]
fn test_restore_review_updates_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectK");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);
    client.hide_review(&project_id, &reviewer, &admin);

    let stats_hidden = client.get_project_stats(&project_id);
    assert_eq!(stats_hidden.review_count, 0);

    client.restore_review(&project_id, &reviewer, &admin);

    let stats_restored = client.get_project_stats(&project_id);
    assert_eq!(stats_restored.review_count, 1);
    assert_eq!(stats_restored.rating_sum, 500);
    assert_eq!(stats_restored.average_rating, 500);
}

#[test]
fn test_restore_review_not_hidden_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectL");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    let result = client.try_restore_review(&project_id, &reviewer, &admin);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotHidden.into())));
}

#[test]
fn test_restore_review_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectM");

    let reviewer = Address::generate(&env);
    let non_admin = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);
    client.hide_review(&project_id, &reviewer, &admin);

    let result = client.try_restore_review(&project_id, &reviewer, &non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly.into())));
}

#[test]
fn test_restore_review_nonexistent_review_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectN");

    let reviewer = Address::generate(&env);

    let result = client.try_restore_review(&project_id, &reviewer, &admin);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound.into())));
}

// ---------------------------------------------------------------------------
// list_reviews excludes hidden reviews
// ---------------------------------------------------------------------------

#[test]
fn test_list_reviews_excludes_hidden() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectO");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    client.add_review(&project_id, &r1, &5, &None);
    client.add_review(&project_id, &r2, &4, &None);
    client.add_review(&project_id, &r3, &3, &None);

    client.hide_review(&project_id, &r2, &admin);

    let reviews = client.list_reviews(&project_id, &0, &100);
    assert_eq!(reviews.len(), 2);

    // Verify the hidden review is not in the list
    for review in reviews.iter() {
        assert_ne!(review.reviewer, r2);
    }
}

#[test]
fn test_list_reviews_all_hidden() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectP");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.add_review(&project_id, &r1, &5, &None);
    client.add_review(&project_id, &r2, &4, &None);

    client.hide_review(&project_id, &r1, &admin);
    client.hide_review(&project_id, &r2, &admin);

    let reviews = client.list_reviews(&project_id, &0, &100);
    assert_eq!(reviews.len(), 0);
}

// ---------------------------------------------------------------------------
// Complex scenarios: multiple operations
// ---------------------------------------------------------------------------

#[test]
fn test_hide_restore_hide_cycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectQ");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    // Hide
    client.hide_review(&project_id, &reviewer, &admin);
    let review1 = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review1.hidden, true);

    // Restore
    client.restore_review(&project_id, &reviewer, &admin);
    let review2 = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review2.hidden, false);

    // Hide again
    client.hide_review(&project_id, &reviewer, &admin);
    let review3 = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review3.hidden, true);
}

#[test]
fn test_report_then_hide() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectR");

    let reviewer = Address::generate(&env);
    let reporter1 = Address::generate(&env);
    let reporter2 = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    client.report_review(&project_id, &reviewer, &reporter1);
    client.report_review(&project_id, &reviewer, &reporter2);

    let review_before = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review_before.report_count, 2);
    assert_eq!(review_before.hidden, false);

    client.hide_review(&project_id, &reviewer, &admin);

    let review_after = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review_after.report_count, 2); // Report count preserved
    assert_eq!(review_after.hidden, true);
}

#[test]
fn test_stats_with_mixed_hidden_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectS");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let r4 = Address::generate(&env);

    client.add_review(&project_id, &r1, &5, &None);
    client.add_review(&project_id, &r2, &4, &None);
    client.add_review(&project_id, &r3, &3, &None);
    client.add_review(&project_id, &r4, &2, &None);

    let stats_all = client.get_project_stats(&project_id);
    assert_eq!(stats_all.review_count, 4);
    assert_eq!(stats_all.rating_sum, 1400); // (5+4+3+2)*100
    assert_eq!(stats_all.average_rating, 350); // 3.5

    // Hide two reviews
    client.hide_review(&project_id, &r1, &admin);
    client.hide_review(&project_id, &r3, &admin);

    let stats_partial = client.get_project_stats(&project_id);
    assert_eq!(stats_partial.review_count, 2);
    assert_eq!(stats_partial.rating_sum, 600); // (4+2)*100
    assert_eq!(stats_partial.average_rating, 300); // 3.0

    // Restore one
    client.restore_review(&project_id, &r1, &admin);

    let stats_restored = client.get_project_stats(&project_id);
    assert_eq!(stats_restored.review_count, 3);
    assert_eq!(stats_restored.rating_sum, 1100); // (5+4+2)*100
    assert_eq!(stats_restored.average_rating, 366); // ~3.67
}

#[test]
fn test_get_review_returns_hidden_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectT");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);
    client.hide_review(&project_id, &reviewer, &admin);

    // get_review should still return the hidden review (for admin access)
    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.hidden, true);
    assert_eq!(review.rating, 5);
}

#[test]
fn test_multiple_projects_independent_moderation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project1 = create_test_project(&client, &admin, "ProjectU");
    let project2 = create_test_project(&client, &admin, "ProjectV");

    let reviewer = Address::generate(&env);
    client.add_review(&project1, &reviewer, &5, &None);
    client.add_review(&project2, &reviewer, &4, &None);

    // Hide review on project1
    client.hide_review(&project1, &reviewer, &admin);

    // Project1 stats should exclude hidden review
    let stats1 = client.get_project_stats(&project1);
    assert_eq!(stats1.review_count, 0);

    // Project2 stats should still include review
    let stats2 = client.get_project_stats(&project2);
    assert_eq!(stats2.review_count, 1);
    assert_eq!(stats2.rating_sum, 400);
}



