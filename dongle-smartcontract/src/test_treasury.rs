use crate::{DongleContract, DongleContractClient};
use crate::types::{VerificationStatus, FeeConfig};
use crate::errors::ContractError;
use soroban_sdk::{testutils::{Address as _, Events}, Address, Env, String, vec};
use soroban_sdk::token;

fn setup_env<'a>(env: &Env) -> (Env, DongleContractClient<'a>, Address) {
    env.mock_all_auths();
    
    let admin = Address::generate(env);
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(env, &contract_id);
    
    client.initialize(&admin);
    
    (env.clone(), client, admin)
}

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let contract_id = env.register_stellar_asset_contract(admin.clone());
    token::Client::new(env, &contract_id)
}

#[test]
fn test_fee_configuration() {
    let env = Env::default();
    let (_, client, admin) = setup_env(&env);
    let token_admin = Address::generate(&env);
    let token = create_token_contract(&env, &token_admin);
    
    client.set_fee_config(&admin, &Some(token.address.clone()), &1000, &0);
    
    let config = client.get_fee_config();
    assert_eq!(config.token, Some(token.address));
    assert_eq!(config.verification_fee, 1000);
    assert_eq!(config.registration_fee, 0);
}

#[test]
fn test_verification_fee_collection() {
    let env = Env::default();
    let (_, client, admin) = setup_env(&env);
    let token_admin = Address::generate(&env);
    let token = create_token_contract(&env, &token_admin);
    
    client.set_fee_config(&admin, &Some(token.address.clone()), &1000, &0);
    
    let user = Address::generate(&env);
    let asset_client = token::StellarAssetClient::new(&env, &token.address);
    asset_client.mint(&user, &5000);
    
    let project_id = client.register_project(
        &user,
        &String::from_str(&env, "Test Project"),
        &String::from_str(&env, "Description"),
        &String::from_str(&env, "Category"),
        &None,
        &None,
        &None,
    );
    
    client.request_verification(&project_id, &user, &String::from_str(&env, "evidence"));
    
    // Check balances
    assert_eq!(token.balance(&user), 4000);
    assert_eq!(token.balance(&client.address), 1000);
    assert_eq!(client.get_treasury_balance(&token.address), 1000);
}

#[test]
fn test_treasury_withdrawal() {
    let env = Env::default();
    let (_, client, admin) = setup_env(&env);
    let token_admin = Address::generate(&env);
    let token = create_token_contract(&env, &token_admin);
    
    client.set_fee_config(&admin, &Some(token.address.clone()), &1000, &0);
    
    let user = Address::generate(&env);
    let asset_client = token::StellarAssetClient::new(&env, &token.address);
    asset_client.mint(&user, &1000);
    
    let project_id = client.register_project(
        &user,
        &String::from_str(&env, "Test Project"),
        &String::from_str(&env, "Description"),
        &String::from_str(&env, "Category"),
        &None,
        &None,
        &None,
    );
    
    client.request_verification(&project_id, &user, &String::from_str(&env, "evidence"));
    
    let treasury_dest = Address::generate(&env);
    client.withdraw_treasury(&admin, &token.address, &600, &treasury_dest);
    
    assert_eq!(token.balance(&treasury_dest), 600);
    assert_eq!(token.balance(&client.address), 400);
    assert_eq!(client.get_treasury_balance(&token.address), 400);
}

#[test]
fn test_insufficient_treasury_funds() {
    let env = Env::default();
    let (_, client, admin) = setup_env(&env);
    let token_admin = Address::generate(&env);
    let token = create_token_contract(&env, &token_admin);
    
    client.set_fee_config(&admin, &Some(token.address.clone()), &1000, &0);
    
    let user = Address::generate(&env);
    let asset_client = token::StellarAssetClient::new(&env, &token.address);
    asset_client.mint(&user, &1000);
    
    let project_id = client.register_project(
        &user, 
        &String::from_str(&env, "P"), 
        &String::from_str(&env, "D"), 
        &String::from_str(&env, "C"), 
        &None, 
        &None, 
        &None
    );
    client.request_verification(&project_id, &user, &String::from_str(&env, "E"));
    
    let treasury_dest = Address::generate(&env);
    let result = client.try_withdraw_treasury(&admin, &token.address, &1500, &treasury_dest);
    
    assert!(result.is_err());
}

#[test]
fn test_unauthorized_withdrawal() {
    let env = Env::default();
    let (_, client, admin) = setup_env(&env);
    let token_admin = Address::generate(&env);
    let token = create_token_contract(&env, &token_admin);
    
    client.set_fee_config(&admin, &Some(token.address.clone()), &1000, &0);
    
    let non_admin = Address::generate(&env);
    let treasury_dest = Address::generate(&env);
    
    let result = client.try_withdraw_treasury(&non_admin, &token.address, &100, &treasury_dest);
    assert!(result.is_err());
}

#[test]
fn test_set_treasury_address() {
    let env = Env::default();
    let (_, client, admin) = setup_env(&env);
    
    let treasury = Address::generate(&env);
    client.set_treasury(&admin, &treasury);
    
    assert_eq!(client.get_treasury(), treasury);
}

#[test]
fn test_unauthorized_set_treasury() {
    let env = Env::default();
    let (_, client, _) = setup_env(&env);
    
    let non_admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    
    let result = client.try_set_treasury(&non_admin, &treasury);
    assert!(result.is_err());
}
