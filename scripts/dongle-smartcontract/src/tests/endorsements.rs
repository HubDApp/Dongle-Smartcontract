#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_endorse_project() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "EndorsableProject");

    client.endorse_project(&project_id, &user);

    let count = client.get_endorsement_count(&project_id);
    assert_eq!(count, 1);

    let endorsed = client.has_endorsed(&project_id, &user);
    assert!(endorsed);
}

#[test]
fn test_unendorse_project() {
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

    let endorsed = client.has_endorsed(&project_id, &user);
    assert!(!endorsed);
}

#[test]
fn test_duplicate_endorse_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "DupEndorseProject");

    client.endorse_project(&project_id, &user);

    let result = client.try_endorse_project(&project_id, &user);
    assert!(result.is_err());
}

#[test]
fn test_endorse_nonexistent_project_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_endorse_project(&999u64, &user);
    assert!(result.is_err());
}

#[test]
fn test_unendorse_without_endorsement_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "NoEndorseYet");

    let result = client.try_unendorse_project(&project_id, &user);
    assert!(result.is_err());
}

#[test]
fn test_endorse_count_multiple_users() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MultiEndorseProject");

    let user_count = 5u32;
    for _ in 0..user_count {
        let u = Address::generate(&env);
        client.endorse_project(&project_id, &u);
    }

    assert_eq!(
        client.get_endorsement_count(&project_id),
        user_count,
        "endorsement count should match total unique endorsers"
    );
}

#[test]
fn test_endorse_after_unendorse_allows_reendorse() {
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

    let endorsed = client.has_endorsed(&project_id, &user);
    assert!(endorsed);
}
