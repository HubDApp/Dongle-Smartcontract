//! Unauthorized access tests for every mutating endpoint.
//!
//! Each test verifies that calling a mutating function with the wrong
//! caller (non-owner, non-admin, or wrong reviewer) returns the correct
//! ContractError and leaves contract state unchanged.

use crate::errors::ContractError;
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn register_project(client: &DongleContractClient, owner: &Address, name: &str) -> u64 {
    let env = &client.env;
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            description: String::from_str(env, "Description"),
            category: String::from_str(env, "DeFi"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
        })
}

fn setup_with_token(
    env: &Env,
    client: &DongleContractClient,
    admin: &Address,
    owner: &Address,
    fee: i128,
) -> (Address, u64) {
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(env, &token_address);
    token_client.mint(owner, &(fee * 10));

    client
        .mock_all_auths()
        .set_fee(admin, &Some(token_address.clone()), &(fee as u128), admin);

    let project_id = register_project(client, owner, "TokenProject");
    client
        .mock_all_auths()
        .pay_fee(owner, &project_id, &Some(token_address.clone()));

    (token_address, project_id)
}

// ── Admin management ──────────────────────────────────────────────────────────

#[test]
fn test_add_admin_by_non_admin_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let non_admin = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.mock_all_auths().try_add_admin(&non_admin, &target);

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
    assert!(!client.is_admin(&target));
}

#[test]
fn test_remove_admin_by_non_admin_fails() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let non_admin = Address::generate(&env);

    // Add a second admin so removal would be valid if authorized
    client.mock_all_auths().add_admin(&admin, &non_admin);
    let third = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &third);

    // non_admin tries to remove admin — but non_admin is not an admin
    let stranger = Address::generate(&env);
    let result = client.mock_all_auths().try_remove_admin(&stranger, &admin);

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
    assert!(client.is_admin(&admin));
}

// ── Project registry ──────────────────────────────────────────────────────────

#[test]
fn test_update_project_by_non_owner_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);

    let project_id = register_project(&client, &owner, "MyProject");

    let params = ProjectUpdateParams {
        project_id,
        caller: attacker.clone(),
        name: Some(String::from_str(&env, "Hijacked")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = client.mock_all_auths().try_update_project(&params);

    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));

    // Project name must be unchanged
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.name, String::from_str(&env, "MyProject"));
}

#[test]
fn test_update_project_nonexistent_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let caller = Address::generate(&env);

    let params = ProjectUpdateParams {
        project_id: 999,
        caller: caller.clone(),
        name: Some(String::from_str(&env, "Ghost")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = client.mock_all_auths().try_update_project(&params);

    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

// ── Review registry ───────────────────────────────────────────────────────────

#[test]
fn test_update_review_by_non_reviewer_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let attacker = Address::generate(&env);

    let project_id = register_project(&client, &owner, "ReviewProject");
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &4, &None);

    // attacker tries to update reviewer's review
    let result = client
        .mock_all_auths()
        .try_update_review(&project_id, &attacker, &5, &None);

    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound)));

    // Original review must be unchanged
    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 4);
}

#[test]
fn test_delete_review_by_non_reviewer_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let attacker = Address::generate(&env);

    let project_id = register_project(&client, &owner, "DeleteProject");
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &3, &None);

    let result = client
        .mock_all_auths()
        .try_delete_review(&project_id, &attacker);

    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound)));

    // Review must still exist
    assert!(client.get_review(&project_id, &reviewer).is_some());
}

#[test]
fn test_add_duplicate_review_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let project_id = register_project(&client, &owner, "DupReview");
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5, &None);

    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &reviewer, &3, &None);

    assert_eq!(result, Err(Ok(ContractError::DuplicateReview)));
}

#[test]
fn test_update_nonexistent_review_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let project_id = register_project(&client, &owner, "NoReview");

    let result = client
        .mock_all_auths()
        .try_update_review(&project_id, &reviewer, &5, &None);

    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound)));
}

#[test]
fn test_delete_nonexistent_review_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let project_id = register_project(&client, &owner, "NoReviewDel");

    let result = client
        .mock_all_auths()
        .try_delete_review(&project_id, &reviewer);

    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound)));
}

// ── Fee manager ───────────────────────────────────────────────────────────────

#[test]
fn test_set_fee_by_non_admin_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let non_admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    let result = client
        .mock_all_auths()
        .try_set_fee(&non_admin, &None, &500u128, &treasury);

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_set_fee_by_admin_succeeds() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    client
        .mock_all_auths()
        .set_fee(&admin, &None, &1000u128, &treasury);

    let config = client.get_fee_config();
    assert_eq!(config.verification_fee, 1000u128);
}

// ── Verification registry ─────────────────────────────────────────────────────

#[test]
fn test_request_verification_by_non_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);

    let (_token_address, project_id) = setup_with_token(&env, &client, &admin, &owner, 100);

    // attacker tries to request verification for owner's project
    let result = client.try_request_verification(
        &project_id,
        &attacker,
        &String::from_str(&env, "ipfs://fake"),
    );

    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_request_verification_for_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let requester = Address::generate(&env);

    let result = client.try_request_verification(
        &999u64,
        &requester,
        &String::from_str(&env, "ipfs://evidence"),
    );

    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_approve_verification_by_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let non_admin = Address::generate(&env);

    let (_, project_id) = setup_with_token(&env, &client, &admin, &owner, 100);

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    let result = client.try_approve_verification(&project_id, &non_admin);

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_reject_verification_by_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let non_admin = Address::generate(&env);

    let (_, project_id) = setup_with_token(&env, &client, &admin, &owner, 100);

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    let result = client.try_reject_verification(&project_id, &non_admin);

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_approve_verification_without_pending_request_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    register_project(&client, &owner, "NoPending");
    let project_id = 1u64;

    // No verification record exists yet
    let result = client.try_approve_verification(&project_id, &admin);

    assert_eq!(result, Err(Ok(ContractError::VerificationNotFound)));
}

#[test]
fn test_reject_verification_without_pending_request_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    register_project(&client, &owner, "NoPendingRej");
    let project_id = 1u64;

    let result = client.try_reject_verification(&project_id, &admin);

    assert_eq!(result, Err(Ok(ContractError::VerificationNotFound)));
}

#[test]
fn test_request_verification_without_fee_payment_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let treasury = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    client.set_fee(&admin, &Some(token_address.clone()), &100u128, &treasury);

    let project_id = register_project(&client, &owner, "NoFeeProject");

    // Do NOT pay fee — request should fail
    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    assert_eq!(result, Err(Ok(ContractError::InsufficientFee)));
}

// ── Cross-endpoint state integrity ────────────────────────────────────────────

#[test]
fn test_unauthorized_update_does_not_mutate_state() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);

    let project_id = register_project(&client, &owner, "Immutable");
    let original = client.get_project(&project_id).unwrap();

    let params = ProjectUpdateParams {
        project_id,
        caller: attacker.clone(),
        name: Some(String::from_str(&env, "Mutated")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let _ = client.mock_all_auths().try_update_project(&params);

    let after = client.get_project(&project_id).unwrap();
    assert_eq!(original.name, after.name);
    assert_eq!(original.updated_at, after.updated_at);
}

#[test]
fn test_unauthorized_review_delete_does_not_mutate_state() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let attacker = Address::generate(&env);

    let project_id = register_project(&client, &owner, "SafeReview");
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5, &None);

    let _ = client
        .mock_all_auths()
        .try_delete_review(&project_id, &attacker);

    // Review must still exist with original rating
    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 5);
}

#[test]
fn test_owner_can_update_own_project() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "OwnProject");

    let params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "UpdatedName")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = client.mock_all_auths().update_project(&params);

    assert_eq!(result.name, String::from_str(&env, "UpdatedName"));
}

#[test]
fn test_reviewer_can_update_own_review() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let project_id = register_project(&client, &owner, "ReviewOwner");
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &2, &None);

    client
        .mock_all_auths()
        .update_review(&project_id, &reviewer, &5, &None);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 5);
}

#[test]
fn test_reviewer_can_delete_own_review() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let project_id = register_project(&client, &owner, "DeleteOwn");
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &3, &None);

    client
        .mock_all_auths()
        .delete_review(&project_id, &reviewer);

    assert!(client.get_review(&project_id, &reviewer).is_none());
}
