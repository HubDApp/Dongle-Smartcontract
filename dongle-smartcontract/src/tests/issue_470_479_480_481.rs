#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract, setup_with_fees};
use crate::types::{ProjectRegistrationParams, VerificationStatus};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

extern crate alloc;
use alloc::format;

fn valid_cid(env: &Env) -> String {
    String::from_str(env, "QmYwAPJzv5CZsnAzt8auVTL1b1WrYbGqD6oZX6mE6xPD58")
}

fn setup_with_token_fee(env: &Env, fee: u128) -> (crate::DongleContractClient<'_>, Address, Address) {
    let (client, admin) = setup_contract(env);
    let token_admin = Address::generate(env);
    let token = env.register_stellar_asset_contract_v2(token_admin).address();
    client.set_fee(&admin, &Some(token.clone()), &fee, &0u128, &admin);
    (client, admin, token)
}

#[test]
fn pay_fee_rejects_archived_or_active_verification_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token) = setup_with_token_fee(&env, 1000);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "FeeProject");
    soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&owner, &5000);

    client.archive_project(&project_id, &owner);
    let archived_result = client.try_pay_fee(&owner, &project_id, &Some(token.clone()));
    assert_eq!(archived_result, Err(Ok(ContractError::InvalidStatus)));

    client.reactivate_project(&project_id, &admin);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(&project_id, &owner, &valid_cid(&env));

    let pending_result = client.try_pay_fee(&owner, &project_id, &Some(token));
    assert_eq!(pending_result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn pending_verification_index_tracks_request_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token) = setup_with_token_fee(&env, 1000);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "PendingIndex");
    soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&owner, &5000);

    client.pay_fee(&owner, &project_id, &Some(token));
    client.request_verification(&project_id, &owner, &valid_cid(&env));

    let pending = client.get_pending_projects(&0, &10);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending.get(0).unwrap().id, project_id);
    assert_eq!(pending.get(0).unwrap().verification_status, VerificationStatus::Pending);

    client.approve_verification(&project_id, &admin);
    assert_eq!(client.get_pending_projects(&0, &10).len(), 0);
}

#[test]
fn report_project_enforces_global_per_user_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let reporter = Address::generate(&env);
    let reason = String::from_str(&env, "QmReportReason123456789012345678901234567890123456");

    for i in 0..100u32 {
        let owner = Address::generate(&env);
        let name = String::from_str(&env, &format!("Project{i}"));
        let slug = String::from_str(&env, &format!("project-{i}"));
        let params = ProjectRegistrationParams {
            owner,
            name,
            slug,
            description: String::from_str(&env, "Test project description"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
        };
        let project_id = client.register_project(&params);
        assert!(client.try_report_project(&project_id, &reporter, &reason).is_ok());
    }

    let extra_owner = Address::generate(&env);
    let extra_project = create_test_project(&client, &extra_owner, "ProjectOverflow");
    let result = client.try_report_project(&extra_project, &reporter, &reason);
    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded)));
}

#[test]
fn register_projects_batch_registers_multiple_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner_a = Address::generate(&env);
    let owner_b = Address::generate(&env);

    let mut batch = Vec::new(&env);
    for (owner, name, slug) in [
        (owner_a.clone(), "BatchOne", "batchone"),
        (owner_b.clone(), "BatchTwo", "batchtwo"),
    ] {
        batch.push_back(ProjectRegistrationParams {
            owner,
            name: String::from_str(&env, name),
            slug: String::from_str(&env, slug),
            description: String::from_str(&env, "Batch project"),
            category: String::from_str(&env, "Infra"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
        });
    }

    let ids = client.register_projects_batch(&batch);
    assert_eq!(ids.len(), 2);
    assert_eq!(client.get_project_count(), 2);
    assert_eq!(client.get_project(&ids.get(0).unwrap()).unwrap().slug, String::from_str(&env, "batchone"));
    assert_eq!(client.get_project(&ids.get(1).unwrap()).unwrap().slug, String::from_str(&env, "batchtwo"));
}
