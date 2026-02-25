use crate::{DongleContract, DongleContractClient};
use crate::types::{VerificationStatus};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address) {
    let admin = Address::generate(env);
    let owner = Address::generate(env);
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(env, &contract_id);
    client.initialize(&admin);
    (client, admin, owner)
}

#[test]
fn test_register_project_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, owner) = setup(&env);
    
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
fn test_update_project_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, owner) = setup(&env);
    
    let id = client.register_project(
        &owner,
        &String::from_str(&env, "Name"),
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "Cat"),
        &None,
        &None,
        &None,
    );
    
    let new_name = String::from_str(&env, "New Name");
    client.update_project(&id, &owner, &Some(new_name.clone()), &None, &None, &None, &None, &None);
    
    let project = client.get_project(&id);
    assert_eq!(project.name, new_name);
}

#[test]
fn test_add_review_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, owner) = setup(&env);
    
    let id = client.register_project(
        &owner,
        &String::from_str(&env, "Name"),
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "Cat"),
        &None,
        &None,
        &None,
    );
    
    let reviewer = Address::generate(&env);
    client.add_review(&id, &reviewer, &5, &None);
    
    let review = client.get_review(&id, &reviewer);
    assert_eq!(review.rating, 5);
}

#[test]
fn test_get_project_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    
    let id = client.register_project(
        &owner,
        &String::from_str(&env, "Project"),
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "Cat"),
        &None, &None, &None
    );

    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);

    client.add_review(&id, &reviewer1, &5, &None);
    client.add_review(&id, &reviewer2, &4, &None);

    let reviews = client.get_project_reviews(&id);
    assert_eq!(reviews.len(), 2);
}

#[test]
fn test_verification_status_sync() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    
    // Setup token and fees
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let _token_client = soroban_sdk::token::Client::new(&env, &token_id);
    let asset_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    
    client.set_fee_config(&admin, &Some(token_id), &100, &50);
    
    let owner = Address::generate(&env);
    asset_client.mint(&owner, &1000);

    let id = client.register_project(
        &owner,
        &String::from_str(&env, "Project"),
        &String::from_str(&env, "Desc"),
        &String::from_str(&env, "Cat"),
        &None, &None, &None
    );

    client.request_verification(&id, &owner, &String::from_str(&env, "Evidence"));
    
    // Initial status should be Unverified in Project struct
    let _project = client.get_project(&id);
    assert_eq!(_project.verification_status, VerificationStatus::Unverified);
    
    client.approve_verification(&id, &admin);
    
    let _updated_project = client.get_project(&id);
    assert_eq!(_updated_project.verification_status, VerificationStatus::Verified);
    
    let verification = client.get_verification(&id);
    assert_eq!(verification.status, VerificationStatus::Verified);
}
