//! Tests for two-step project ownership transfer.

use crate::errors::ContractError;
use crate::types::ProjectRegistrationParams;
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

fn register(client: &DongleContractClient<'_>, env: &Env, owner: &Address, name: &str) -> u64 {
    client.register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        description: String::from_str(env, "A test project description here"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    })
}

// --- Happy path ---

#[test]
fn test_full_transfer_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let old_owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let project_id = register(&client, &env, &old_owner, "My Project");

    // Step 1: initiate
    client.initiate_transfer(&project_id, &old_owner, &new_owner);

    // Project owner unchanged until accepted
    assert_eq!(client.get_project(&project_id).unwrap().owner, old_owner);

    // Step 2: accept
    client.accept_transfer(&project_id, &new_owner);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.owner, new_owner);
}

#[test]
fn test_owner_indexes_updated_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let old_owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let project_id = register(&client, &env, &old_owner, "Index Project");

    client.initiate_transfer(&project_id, &old_owner, &new_owner);
    client.accept_transfer(&project_id, &new_owner);

    // Old owner no longer has the project
    let old_projects = client.get_projects_by_owner(&old_owner);
    assert_eq!(old_projects.len(), 0);

    // New owner now has the project
    let new_projects = client.get_projects_by_owner(&new_owner);
    assert_eq!(new_projects.len(), 1);
    assert_eq!(new_projects.get(0).unwrap().id, project_id);
}

#[test]
fn test_old_owner_retains_other_projects_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let old_owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let id1 = register(&client, &env, &old_owner, "Project A");
    let id2 = register(&client, &env, &old_owner, "Project B");

    client.initiate_transfer(&id1, &old_owner, &new_owner);
    client.accept_transfer(&id1, &new_owner);

    let old_projects = client.get_projects_by_owner(&old_owner);
    assert_eq!(old_projects.len(), 1);
    assert_eq!(old_projects.get(0).unwrap().id, id2);
}

#[test]
fn test_cancel_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let project_id = register(&client, &env, &owner, "Cancel Project");

    client.initiate_transfer(&project_id, &owner, &new_owner);
    client.cancel_transfer(&project_id, &owner);

    // new_owner can no longer accept
    let result = client.try_accept_transfer(&project_id, &new_owner);
    assert_eq!(result, Err(Ok(ContractError::TransferNotFound)));

    // Owner unchanged
    assert_eq!(client.get_project(&project_id).unwrap().owner, owner);
}

#[test]
fn test_initiate_overwrites_previous_pending_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let first_recipient = Address::generate(&env);
    let second_recipient = Address::generate(&env);
    let project_id = register(&client, &env, &owner, "Overwrite Project");

    client.initiate_transfer(&project_id, &owner, &first_recipient);
    // Owner changes their mind
    client.initiate_transfer(&project_id, &owner, &second_recipient);

    // First recipient can no longer accept
    let result = client.try_accept_transfer(&project_id, &first_recipient);
    assert_eq!(result, Err(Ok(ContractError::NotPendingTransferRecipient)));

    // Second recipient can accept
    client.accept_transfer(&project_id, &second_recipient);
    assert_eq!(
        client.get_project(&project_id).unwrap().owner,
        second_recipient
    );
}

// --- Authorization failures ---

#[test]
fn test_initiate_by_non_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let project_id = register(&client, &env, &owner, "Auth Project");

    let result = client.try_initiate_transfer(&project_id, &attacker, &attacker);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_accept_by_wrong_address_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let intended = Address::generate(&env);
    let attacker = Address::generate(&env);
    let project_id = register(&client, &env, &owner, "Wrong Accept");

    client.initiate_transfer(&project_id, &owner, &intended);

    let result = client.try_accept_transfer(&project_id, &attacker);
    assert_eq!(result, Err(Ok(ContractError::NotPendingTransferRecipient)));
}

#[test]
fn test_cancel_by_non_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let project_id = register(&client, &env, &owner, "Cancel Auth");

    client.initiate_transfer(&project_id, &owner, &new_owner);

    let result = client.try_cancel_transfer(&project_id, &attacker);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

// --- Edge cases ---

#[test]
fn test_initiate_on_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let caller = Address::generate(&env);
    let result = client.try_initiate_transfer(&9999, &caller, &caller);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_accept_with_no_pending_transfer_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = register(&client, &env, &owner, "No Pending");

    let result = client.try_accept_transfer(&project_id, &owner);
    assert_eq!(result, Err(Ok(ContractError::TransferNotFound)));
}

#[test]
fn test_cancel_with_no_pending_transfer_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = register(&client, &env, &owner, "No Cancel");

    let result = client.try_cancel_transfer(&project_id, &owner);
    assert_eq!(result, Err(Ok(ContractError::TransferNotFound)));
}
