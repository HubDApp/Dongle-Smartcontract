use crate::DongleContract;
use crate::DongleContractClient;
use crate::errors::ContractError;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient, Address) {
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(env, &contract_id);
    let owner = Address::generate(env);
    env.mock_all_auths();
    (client, owner)
}

#[test]
fn test_register_project_success() {
    let env = Env::default();
    let (client, owner) = setup(&env);

    let name = String::from_str(&env, "Project A");
    let desc = String::from_str(&env, "Description A");
    let cat = String::from_str(&env, "DeFi");

    let id = client.register_project(
        &owner,
        &name,
        &desc,
        &cat,
        &None,
        &None,
        &None,
    );

    assert_eq!(id, 1);
    
    let project = client.get_project(&id);
    assert_eq!(project.name, name);
    assert_eq!(project.owner, owner);
}

#[test]
fn test_register_duplicate_project_fails() {
    let env = Env::default();
    let (client, owner) = setup(&env);

    let name = String::from_str(&env, "Project A");
    let desc = String::from_str(&env, "Description A");
    let cat = String::from_str(&env, "DeFi");

    // Register first project
    client.register_project(
        &owner,
        &name,
        &desc,
        &cat,
        &None,
        &None,
        &None,
    );

    // Attempt to register another project with the same name
    let result = client.try_register_project(
        &owner,
        &name,
        &desc,
        &cat,
        &None,
        &None,
        &None,
    );

    assert_eq!(result, Err(Ok(ContractError::ProjectAlreadyExists)));
}

#[test]
fn test_register_different_projects_success() {
    let env = Env::default();
    let (client, owner) = setup(&env);

    let name1 = String::from_str(&env, "Project A");
    let name2 = String::from_str(&env, "Project B");
    let desc = String::from_str(&env, "Description");
    let cat = String::from_str(&env, "DeFi");

    let id1 = client.register_project(
        &owner,
        &name1,
        &desc,
        &cat,
        &None,
        &None,
        &None,
    );
    assert_eq!(id1, 1);

    let id2 = client.register_project(
        &owner,
        &name2,
        &desc,
        &cat,
        &None,
        &None,
        &None,
    );
    assert_eq!(id2, 2);
}
