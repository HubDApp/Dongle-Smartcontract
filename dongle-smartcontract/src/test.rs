#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env, String};

#[test]
fn test_valid_name() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProjectRegistry);
    let client = ProjectRegistryClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    
    // Mock Auth
    env.mock_all_auths();
    
    let result = client.register_project(
        &user,
        &String::from_str(&env, "Valid_Project-123"),
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "DeFi"),
        &None,
        &None,
        &None,
    );
    
    assert_eq!(result, 1);
}

#[test]
#[should_panic(expected = "Error(Contract, 1)")]
fn test_empty_name() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProjectRegistry);
    let client = ProjectRegistryClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    
    env.mock_all_auths();
    
    client.register_project(
        &user,
        &String::from_str(&env, ""),
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "DeFi"),
        &None,
        &None,
        &None,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, 1)")]
fn test_whitespace_only_name() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProjectRegistry);
    let client = ProjectRegistryClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    
    env.mock_all_auths();
    
    client.register_project(
        &user,
        &String::from_str(&env, "   "),
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "DeFi"),
        &None,
        &None,
        &None,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, 3)")]
fn test_invalid_chars() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProjectRegistry);
    let client = ProjectRegistryClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    
    env.mock_all_auths();
    
    client.register_project(
        &user,
        &String::from_str(&env, "Project@123"), // @ is invalid
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "DeFi"),
        &None,
        &None,
        &None,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, 2)")]
fn test_exceeds_max_length() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProjectRegistry);
    let client = ProjectRegistryClient::new(&env, &contract_id);
    
    let user = Address::generate(&env);
    
    env.mock_all_auths();
    
    let long_name = "ThisIsAVeryLongProjectNameThatExceedsTheFiftyCharacterLimit";
    client.register_project(
        &user,
        &String::from_str(&env, long_name),
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "DeFi"),
        &None,
        &None,
        &None,
    );
}
