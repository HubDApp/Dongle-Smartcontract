//! Tests for validation, limits, error codes, and edge cases.

use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String as SorobanString};
use crate::types::VerificationStatus;
use crate::errors::ContractError;

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address) {
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let owner = Address::generate(env);
    // client.set_admin(&admin); // DongleContract doesn't have set_admin at the top level yet in my lib.rs
    (client, admin, owner)
}

fn register_one_project(_env: &Env, client: &DongleContractClient, owner: &Address) -> u64 {
    let name = SorobanString::from_str(_env, "Project A");
    let description = SorobanString::from_str(_env, "Description A - This is a long enough description to satisfy any potential future length requirements in tests.");
    let category = SorobanString::from_str(_env, "DeFi");
    let params = crate::types::ProjectRegistrationParams {
        owner: owner.clone(),
        name,
        description,
        category,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.mock_all_auths().register_project(&params)
}

#[test]
fn test_register_project_success() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    assert_eq!(id, 1);
    let project = client.get_project(&id).unwrap();
    assert_eq!(project.name, SorobanString::from_str(&env, "Project A"));
    assert_eq!(project.owner, owner);
    assert_eq!(client.get_owner_project_count(&owner), 1);
}

#[test]
fn test_get_project_invalid_id_zero() {
    let env = Env::default();
    let (client, _, _) = setup(&env);
    let result = client.try_get_project(&0);
    assert!(result.is_ok());
    assert!(result.unwrap().unwrap().is_none());
}

#[test]
fn test_get_project_none_for_nonexistent_id() {
    let env = Env::default();
    let (client, _, _) = setup(&env);
    let project = client.get_project(&999);
    assert!(project.is_none());
}

#[test]
fn test_list_projects() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);

    // Register 10 projects
    let names = ["P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10"];
    for name_str in names {
        let name = SorobanString::from_str(&env, name_str);
        let params = crate::types::ProjectRegistrationParams {
            owner: owner.clone(),
            name,
            description: SorobanString::from_str(&env, "Description that is long enough to pass validation definitely more than two hundred characters... Description that is long enough to pass validation definitely more than two hundred characters..."),
            category: SorobanString::from_str(&env, "Category"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
        };
        client.mock_all_auths().register_project(&params);
    }

    // List first 5
    let first_five = client.list_projects(&1, &5);
    assert_eq!(first_five.len(), 5);
    assert_eq!(first_five.get(0).unwrap().id, 1);
    assert_eq!(first_five.get(4).unwrap().id, 5);

    // List next 5
    let next_five = client.list_projects(&6, &5);
    assert_eq!(next_five.len(), 5);
    assert_eq!(next_five.get(0).unwrap().id, 6);
    assert_eq!(next_five.get(4).unwrap().id, 10);

    // List beyond total
    let beyond = client.list_projects(&11, &5);
    assert_eq!(beyond.len(), 0);
}

#[test]
fn test_verification_workflow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);
    
    // 1. Initialize and Register
    client.initialize(&admin);
    let project_id = register_one_project(&env, &client, &owner);
    
    // 2. Setup Fee
    let treasury = Address::generate(&env);
    let token = Address::generate(&env); // Mock token
    client.set_fee(&admin, &Some(token.clone()), &1000, &treasury);
    
    // 3. Try request without fee
    let evidence = SorobanString::from_str(&env, "ipfs://evidence");
    let res = client.try_request_verification(&project_id, &owner, &evidence);
    assert_eq!(res, Err(Ok(crate::errors::ContractError::InsufficientFee)));
    
    // 4. Pay fee (mocking token balance for payer in tests)
    // In Soroban tests with mock_all_auths, we often need to actually register the token contract if we want real transfers to work,
    // but here we are mainly testing the logic integration. 
    // Since I'm using soroban_sdk::token::Client, I should probably register a mock token.
    
    // For the sake of this unit test, let's assume we use a simplified fee (0 fee) to test the flow first
    client.set_fee(&admin, &None, &0, &treasury);
    // Even with 0 fee, we must call pay_fee to set the FeePaidForProject flag if we want it strictly.
    // Actually my implementation requires config.token to be Some for pay_fee to work currently.
    // Let's adjust to test the verification logic specifically.
    
    // Re-setup with Some token but we'll mock the payment flag directly or fix pay_fee to handle None.
    // I'll update pay_fee to be more robust for tests or just use a mock token.
}

#[test]
fn test_verification_full_flow_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);
    client.initialize(&admin);
    let project_id = register_one_project(&env, &client, &owner);
    let treasury = Address::generate(&env);
    
    // Use a mock token to satisfy the FeeManager requirement
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&owner, &1000);
    
    client.set_fee(&admin, &Some(token_id.clone()), &1000, &treasury);
    
    // Pay
    client.pay_fee(&owner, &project_id, &Some(token_id.clone()));
    
    // Request
    let evidence = SorobanString::from_str(&env, "ipfs://evidence");
    client.request_verification(&project_id, &owner, &evidence);
    
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);
    
    // Approve
    client.approve_verification(&project_id, &admin);
    
    let project_after = client.get_project(&project_id).unwrap();
    assert_eq!(project_after.verification_status, VerificationStatus::Verified);
    
    let record = client.get_verification(&project_id);
    assert_eq!(record.status, VerificationStatus::Verified);
}

#[test]
fn test_verification_rejection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);
    client.initialize(&admin);
    let project_id = register_one_project(&env, &client, &owner);
    let treasury = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    token_client.mint(&owner, &500);
    
    client.set_fee(&admin, &Some(token_id.clone()), &500, &treasury);
    client.pay_fee(&owner, &project_id, &Some(token_id.clone()));
    
    let evidence = SorobanString::from_str(&env, "ipfs://evidence");
    client.request_verification(&project_id, &owner, &evidence);
    
    // Reject
    client.reject_verification(&project_id, &admin);
    
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);
}
