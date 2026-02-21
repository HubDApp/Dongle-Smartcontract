#![cfg(test)]
use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Env, Address};

#[test]
fn test_init() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    // Verify admin is set (via internal storage)
    // In a real test, we might expose a getter or check events
}

#[test]
fn test_register_project() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    let project_id = client.register_project(
        &owner,
        &String::from_str(&env, "Test Project"),
        &String::from_str(&env, "Description"),
        &String::from_str(&env, "Category"),
        &None,
        &None,
        &None,
    );

    assert_eq!(project_id, 0); // Placeholder returns 0
}
