//! Tests for validation, limits, error codes, and edge cases.

use crate::constants::MAX_PROJECTS_PER_USER;
use crate::errors::Error;
use crate::types::{FeeConfig, VerificationStatus};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String as SorobanString};

fn setup(env: &Env) -> (DongleContractClient, Address, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let owner = Address::generate(env);
    client.set_admin(&admin);
    (client, admin, owner)
}

fn register_one_project(
    _env: &Env,
    client: &DongleContractClient,
    owner: &Address,
) -> u64 {
    client.register_project(
        owner,
        &"Project A".into(),
        &"Description A".into(),
        &"DeFi".into(),
        &None,
        &None,
        &None,
    )
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
fn test_validation_invalid_project_name_empty() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let result = client.try_register_project(
        &owner,
        &"".into(),
        &"Desc".into(),
        &"Cat".into(),
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(Error::InvalidProjectName)));
}

#[test]
fn test_validation_invalid_project_name_whitespace_only() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let result = client.try_register_project(
        &owner,
        &"   ".into(),
        &"Desc".into(),
        &"Cat".into(),
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(Error::InvalidProjectName)));
}

#[test]
fn test_validation_invalid_description_empty() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let result = client.try_register_project(
        &owner,
        &"Name".into(),
        &"".into(),
        &"Cat".into(),
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(Error::InvalidProjectDescription)));
}

#[test]
fn test_validation_invalid_category_empty() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let result = client.try_register_project(
        &owner,
        &"Name".into(),
        &"Desc".into(),
        &"".into(),
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(Error::InvalidProjectCategory)));
}

#[test]
fn test_update_project_not_owner_reverts() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let other = Address::generate(&env);
    let result = client.try_update_project(
        &id,
        &other,
        &"Name2".into(),
        &"Desc2".into(),
        &"Cat2".into(),
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(Error::NotProjectOwner)));
}

#[test]
fn test_get_project_invalid_id_zero() {
    let env = Env::default();
    let (client, _, _) = setup(&env);
    let result = client.try_get_project(&0);
    assert_eq!(result, Err(Ok(Error::InvalidProjectId)));
}

#[test]
fn test_max_projects_per_user_limit() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let name = "Project".to_string();
    let desc = "Description".to_string();
    let cat = "DeFi".to_string();
    for i in 0..MAX_PROJECTS_PER_USER {
        let n = format!("{} {}", name, i);
        let id = client.register_project(
            &owner,
            &n,
            &desc,
            &cat,
            &None,
            &None,
            &None,
        );
        assert!(id > 0);
    }
    assert_eq!(client.get_owner_project_count(&owner), MAX_PROJECTS_PER_USER);
    let result = client.try_register_project(
        &owner,
        &"One more".into(),
        &desc,
        &cat,
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(Error::MaxProjectsPerUserExceeded)));
}

#[test]
fn test_add_review_invalid_rating_zero() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&id, &reviewer, &0u32, &None);
    assert_eq!(result, Err(Ok(Error::InvalidRating)));
}

#[test]
fn test_add_review_invalid_rating_six() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&id, &reviewer, &6u32, &None);
    assert_eq!(result, Err(Ok(Error::InvalidRating)));
}

#[test]
fn test_add_review_valid_rating_one_to_five() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let reviewer = Address::generate(&env);
    for r in 1u32..=5 {
        let result = client.try_add_review(&id, &reviewer, &r, &None);
        if r == 1 {
            assert!(result.is_ok(), "first review should succeed");
        } else {
            assert_eq!(result, Err(Ok(Error::DuplicateReview)), "second review same reviewer");
        }
    }
}

#[test]
fn test_duplicate_review_same_reviewer_reverts() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let reviewer = Address::generate(&env);
    client.add_review(&id, &reviewer, &5u32, &None);
    let result = client.try_add_review(&id, &reviewer, &4u32, &None);
    assert_eq!(result, Err(Ok(Error::DuplicateReview)));
}

#[test]
fn test_update_review_not_author_reverts() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let reviewer = Address::generate(&env);
    client.add_review(&id, &reviewer, &5u32, &None);
    let other = Address::generate(&env);
    let result = client.try_update_review(&id, &other, &3u32, &None);
    assert_eq!(result, Err(Ok(Error::ReviewNotFound)));
}

#[test]
fn test_request_verification_without_fee_reverts() {
    let env = Env::default();
    let (client, admin, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let treasury = Address::generate(&env);
    client.set_fee(&admin, &None, &100, &treasury);
    let result = client.try_request_verification(&id, &owner, &"evidence_cid".into());
    assert_eq!(result, Err(Ok(Error::FeeNotPaid)));
}

#[test]
fn test_request_verification_not_owner_reverts() {
    let env = Env::default();
    let (client, admin, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let treasury = Address::generate(&env);
    client.set_fee(&admin, &None, &100, &treasury);
    client.pay_fee(&owner, &id, &None);
    let other = Address::generate(&env);
    let result = client.try_request_verification(&id, &other, &"evidence_cid".into());
    assert_eq!(result, Err(Ok(Error::NotProjectOwnerForVerification)));
}

#[test]
fn test_request_verification_invalid_evidence_empty_reverts() {
    let env = Env::default();
    let (client, admin, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let treasury = Address::generate(&env);
    client.set_fee(&admin, &None, &100, &treasury);
    client.pay_fee(&owner, &id, &None);
    let result = client.try_request_verification(&id, &owner, &"".into());
    assert_eq!(result, Err(Ok(Error::InvalidEvidenceCid)));
}

#[test]
fn test_approve_verification_unauthorized_reverts() {
    let env = Env::default();
    let (client, admin, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let treasury = Address::generate(&env);
    client.set_fee(&admin, &None, &100, &treasury);
    client.pay_fee(&owner, &id, &None);
    client.request_verification(&id, &owner, &"evidence".into());
    let non_admin = Address::generate(&env);
    let result = client.try_approve_verification(&id, &non_admin);
    assert_eq!(result, Err(Ok(Error::UnauthorizedVerifier)));
}

#[test]
fn test_verification_flow_approve() {
    let env = Env::default();
    let (client, admin, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let treasury = Address::generate(&env);
    client.set_fee(&admin, &None, &100, &treasury);
    client.pay_fee(&owner, &id, &None);
    client.request_verification(&id, &owner, &"evidence".into());
    client.approve_verification(&id, &admin);
    let rec = client.get_verification(&id).expect("verification record");
    assert_eq!(rec.status, VerificationStatus::Verified);
}

#[test]
fn test_verification_flow_reject() {
    let env = Env::default();
    let (client, admin, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let treasury = Address::generate(&env);
    client.set_fee(&admin, &None, &100, &treasury);
    client.pay_fee(&owner, &id, &None);
    client.request_verification(&id, &owner, &"evidence".into());
    client.reject_verification(&id, &admin);
    let rec = client.get_verification(&id).expect("verification record");
    assert_eq!(rec.status, VerificationStatus::Rejected);
}

#[test]
fn test_set_fee_unauthorized_reverts() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);
    let treasury = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let result = client.try_set_fee(&non_admin, &None, &100, &treasury);
    assert_eq!(result, Err(Ok(Error::UnauthorizedAdmin)));
    client.set_fee(&admin, &None, &100, &treasury);
}

#[test]
fn test_set_fee_zero_amount_reverts() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);
    let treasury = Address::generate(&env);
    let result = client.try_set_fee(&admin, &None, &0, &treasury);
    assert_eq!(result, Err(Ok(Error::InvalidFeeAmount)));
}

#[test]
fn test_pay_fee_before_config_reverts() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let id = register_one_project(&env, &client, &owner);
    let result = client.try_pay_fee(&owner, &id, &None);
    assert_eq!(result, Err(Ok(Error::FeeNotConfigured)));
}

#[test]
fn test_get_project_none_for_nonexistent_id() {
    let env = Env::default();
    let (client, _, _) = setup(&env);
    let project = client.get_project(&999);
    assert!(project.is_none());
}

#[test]
fn test_multiple_concurrent_registrations_same_user() {
    let env = Env::default();
    let (client, _, owner) = setup(&env);
    let mut ids = Vec::new();
    for i in 0..5 {
        let id = client.register_project(
            &owner,
            &format!("Project {}", i),
            &"Desc".into(),
            &"Cat".into(),
            &None,
            &None,
            &None,
        );
        ids.push(id);
    }
    assert_eq!(ids, [1, 2, 3, 4, 5]);
    assert_eq!(client.get_owner_project_count(&owner), 5);
}

#[test]
fn test_get_fee_config_after_set() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);
    let treasury = Address::generate(&env);
    client.set_fee(&admin, &None, &500, &treasury);
    let config: FeeConfig = client.get_fee_config();
    assert_eq!(config.amount, 500);
    assert_eq!(config.treasury, treasury);
}
