#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::ContractError;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn endorsement_defaults_are_empty() {
    let env = Env::default();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    assert_eq!(client.get_endorsement_count(&999), 0);
    assert!(!client.has_endorsed(&999, &user));
}

#[test]
fn endorse_project_updates_count_and_membership() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "EndorsableProject");

    client.endorse_project(&project_id, &user);

    assert_eq!(client.get_endorsement_count(&project_id), 1);
    assert!(client.has_endorsed(&project_id, &user));
}

#[test]
fn unendorse_project_updates_count_and_membership() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "UnendorsableProject");

    client.endorse_project(&project_id, &user);
    assert_eq!(client.get_endorsement_count(&project_id), 1);

    client.unendorse_project(&project_id, &user);
    assert_eq!(client.get_endorsement_count(&project_id), 0);

    assert!(!client.has_endorsed(&project_id, &user));
}

#[test]
fn duplicate_endorse_returns_exact_error_without_changing_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "DupEndorseProject");

    client.endorse_project(&project_id, &user);

    let result = client.try_endorse_project(&project_id, &user);
    assert_eq!(result, Err(Ok(ContractError::AlreadyEndorsed)));
    assert_eq!(client.get_endorsement_count(&project_id), 1);
    assert!(client.has_endorsed(&project_id, &user));
}

#[test]
fn endorse_nonexistent_project_returns_exact_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_endorse_project(&999u64, &user);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn unendorse_without_endorsement_returns_exact_error_without_changing_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "NoEndorseYet");

    let result = client.try_unendorse_project(&project_id, &user);
    assert_eq!(result, Err(Ok(ContractError::NotEndorsed)));
    assert_eq!(client.get_endorsement_count(&project_id), 0);
    assert!(!client.has_endorsed(&project_id, &user));
}

#[test]
fn duplicate_unendorse_returns_exact_error_without_changing_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "DoubleUnendorseProject");

    client.endorse_project(&project_id, &user);
    client.unendorse_project(&project_id, &user);

    let result = client.try_unendorse_project(&project_id, &user);
    assert_eq!(result, Err(Ok(ContractError::NotEndorsed)));
    assert_eq!(client.get_endorsement_count(&project_id), 0);
    assert!(!client.has_endorsed(&project_id, &user));
}

#[test]
fn endorsement_count_tracks_multiple_users_and_partial_removal() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MultiEndorseProject");

    let first_user = Address::generate(&env);
    let second_user = Address::generate(&env);
    let third_user = Address::generate(&env);

    client.endorse_project(&project_id, &first_user);
    client.endorse_project(&project_id, &second_user);
    client.endorse_project(&project_id, &third_user);

    assert_eq!(client.get_endorsement_count(&project_id), 3);
    assert!(client.has_endorsed(&project_id, &first_user));
    assert!(client.has_endorsed(&project_id, &second_user));
    assert!(client.has_endorsed(&project_id, &third_user));

    client.unendorse_project(&project_id, &second_user);

    assert_eq!(client.get_endorsement_count(&project_id), 2);
    assert!(client.has_endorsed(&project_id, &first_user));
    assert!(!client.has_endorsed(&project_id, &second_user));
    assert!(client.has_endorsed(&project_id, &third_user));
}

#[test]
fn endorsement_state_is_isolated_by_project_and_user() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let endorsing_user = Address::generate(&env);
    let other_user = Address::generate(&env);

    let first_project = create_test_project(&client, &owner, "FirstEndorsementProject");
    let second_project = create_test_project(&client, &owner, "SecondEndorsementProject");

    client.endorse_project(&first_project, &endorsing_user);

    assert_eq!(client.get_endorsement_count(&first_project), 1);
    assert_eq!(client.get_endorsement_count(&second_project), 0);
    assert!(client.has_endorsed(&first_project, &endorsing_user));
    assert!(!client.has_endorsed(&first_project, &other_user));
    assert!(!client.has_endorsed(&second_project, &endorsing_user));
}

#[test]
fn project_owner_can_self_endorse() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "SelfEndorseProject");

    client.endorse_project(&project_id, &owner);

    assert_eq!(client.get_endorsement_count(&project_id), 1);
    assert!(client.has_endorsed(&project_id, &owner));
}

#[test]
fn endorse_after_unendorse_allows_reendorsement() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ReendorseProject");

    client.endorse_project(&project_id, &user);
    client.unendorse_project(&project_id, &user);
    assert_eq!(client.get_endorsement_count(&project_id), 0);

    client.endorse_project(&project_id, &user);
    assert_eq!(client.get_endorsement_count(&project_id), 1);

    assert!(client.has_endorsed(&project_id, &user));
}
