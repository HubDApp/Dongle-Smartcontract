//! Reusable test fixtures and helper functions for contract testing.

#![allow(dead_code)]

use crate::types::{Project, ProjectRegistrationParams, VerificationStatus};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

pub fn setup_contract(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

pub fn generate_test_users(env: &Env, count: u32) -> Vec<Address> {
    let mut users = Vec::new(env);
    for _ in 0..count {
        users.push_back(Address::generate(env));
    }
    users
}

pub fn setup_with_fees(
    env: &Env,
    fee_amount: u128,
) -> (DongleContractClient<'_>, Address, Address) {
    let (client, admin) = setup_contract(env);
    let treasury = Address::generate(env);
    client
        .mock_all_auths()
        .set_fee(&admin, &None, &fee_amount, &treasury);
    (client, admin, treasury)
}

pub fn create_test_project(client: &DongleContractClient, owner: &Address, name: &str) -> u64 {
    let env = &client.env;
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        description: String::from_str(env, "Test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.register_project(&params)
}

pub fn create_project_with_reviews(
    env: &Env,
    client: &DongleContractClient,
    review_count: u32,
) -> (u64, Vec<Address>) {
    let owner = Address::generate(env);
    let project_id = create_test_project(client, &owner, "Test Project");
    let reviewers = generate_test_users(env, review_count);
    for (i, reviewer) in reviewers.iter().enumerate() {
        let rating = ((i % 5) + 1) as u32;
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &rating, &None);
    }
    (project_id, reviewers)
}

pub fn assert_project_state(
    project: &Project,
    expected_name: &str,
    expected_owner: &Address,
    expected_status: VerificationStatus,
) {
    let env = &project.name.env();
    assert_eq!(project.name, String::from_str(env, expected_name));
    assert_eq!(project.owner, *expected_owner);
    assert_eq!(project.verification_status, expected_status);
}

/// Get a test environment (convenience function).
pub fn get_test_env() -> Env {
    Env::default()
}

/// Get a test admin address.
pub fn get_admin(env: &Env) -> Address {
    Address::generate(env)
}

/// Get a test user address.
pub fn get_user(env: &Env) -> Address {
    Address::generate(env)
}
