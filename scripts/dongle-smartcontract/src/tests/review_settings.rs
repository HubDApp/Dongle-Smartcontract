//! Tests for per-project review settings (issue #129).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_reviews_enabled_by_default() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ProjectA");

    assert!(client.get_reviews_enabled(&project_id));
}

#[test]
fn test_owner_can_disable_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ProjectB");

    client.set_reviews_enabled(&project_id, &admin, &false);
    assert!(!client.get_reviews_enabled(&project_id));
}

#[test]
fn test_owner_can_re_enable_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ProjectC");

    client.set_reviews_enabled(&project_id, &admin, &false);
    client.set_reviews_enabled(&project_id, &admin, &true);
    assert!(client.get_reviews_enabled(&project_id));
}

#[test]
fn test_non_owner_cannot_disable_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ProjectD");

    let non_owner = Address::generate(&env);
    let result = client.try_set_reviews_enabled(&project_id, &non_owner, &false);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized.into())));
}

#[test]
fn test_add_review_fails_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ProjectE");

    client.set_reviews_enabled(&project_id, &admin, &false);

    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&project_id, &reviewer, &5, &None);
    assert_eq!(result, Err(Ok(ContractError::ReviewsDisabled.into())));
}

#[test]
fn test_submit_review_fails_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ProjectF");

    client.set_reviews_enabled(&project_id, &admin, &false);

    let reviewer = Address::generate(&env);
    let cid = soroban_sdk::String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    let result = client.try_submit_review(&project_id, &reviewer, &5, &cid);
    assert_eq!(result, Err(Ok(ContractError::ReviewsDisabled.into())));
}

#[test]
fn test_existing_reviews_readable_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ProjectG");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &4, &None);

    // Disable reviews
    client.set_reviews_enabled(&project_id, &admin, &false);

    // Existing review is still readable
    let review = client.get_review(&project_id, &reviewer);
    assert!(review.is_some());
    assert_eq!(review.unwrap().rating, 4);
}

#[test]
fn test_set_reviews_enabled_on_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let caller = Address::generate(&env);
    let result = client.try_set_reviews_enabled(&999, &caller, &false);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound.into())));
}
