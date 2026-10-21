//! Fine-grained pause matrix tests (issue #902).
//!
//! Covers the `PauseState` enum (`Running`, `PausedAll`,
//! `PausedVerification`, `PausedFees`, `PausedReviews`) and verifies for
//! **every** combination which operation families are blocked:
//!
//! * generic mutations (`register_project`),
//! * review submission (`add_review`),
//! * verification requests (`request_verification`),
//! * fee payments (`pay_fee`).
//!
//! The assertions deliberately only look at the *pause* error: the other
//! business rules (reviewer eligibility, fee configuration, …) are out of
//! scope for this matrix.

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{PauseOperation, PauseState, ProjectRegistrationParams};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

type PauseResult = Result<
    Result<(), soroban_sdk::ConversionError>,
    Result<ContractError, soroban_sdk::InvokeError>,
>;

/// Initialise the contract and create a project *before* any pause is
/// applied so that the project exists in every scenario.
fn setup(env: &Env) -> (DongleContractClient<'_>, Address, u64, Address, Address) {
    let (client, admin) = setup_contract(env);
    let owner = Address::generate(env);
    let project_id = create_test_project(&client, &owner, "MatrixProject");
    let reviewer = Address::generate(env);
    (client, admin, project_id, owner, reviewer)
}

/// Try to submit a review — blocked when reviews are paused.
fn try_review(
    client: &DongleContractClient<'_>,
    project_id: u64,
    reviewer: &Address,
) -> PauseResult {
    client
        .mock_all_auths()
        .try_add_review(&project_id, reviewer, &4u32, &None)
}

/// Try to request a verification — blocked when verification is paused.
fn try_verification(
    client: &DongleContractClient<'_>,
    project_id: u64,
    owner: &Address,
) -> PauseResult {
    client.mock_all_auths().try_request_verification(
        &project_id,
        owner,
        &String::from_str(&client.env, "QmEvidenceCidPauseMatrix1234567890"),
    )
}

/// Try to pay a fee — blocked when fees are paused.
fn try_fee(
    client: &DongleContractClient<'_>,
    project_id: u64,
    owner: &Address,
) -> PauseResult {
    client
        .mock_all_auths()
        .try_pay_fee(owner, &project_id, &None)
}

/// Try a generic mutation — blocked by `PausedAll` (and by a staged recovery
/// that has not reached its final phase).
fn try_generic(
    client: &DongleContractClient<'_>,
    owner: &Address,
) -> Result<
    Result<u64, soroban_sdk::ConversionError>,
    Result<ContractError, soroban_sdk::InvokeError>,
> {
    client
        .mock_all_auths()
        .try_register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&client.env, "SecondProject"),
            slug: String::from_str(&client.env, "second-project"),
            description: String::from_str(&client.env, "Another project"),
            category: String::from_str(&client.env, "DeFi"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
            language_code: None,
        })
}

#[test]
fn test_running_blocks_nothing() {
    let env = Env::default();
    let (client, _admin, project_id, owner, reviewer) = setup(&env);

    assert_eq!(client.get_pause_state(), PauseState::Running);
    assert!(!client.is_paused());

    assert_ne!(
        try_review(&client, project_id, &reviewer),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_verification(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_fee(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_paused_all_blocks_every_operation_family() {
    let env = Env::default();
    let (client, admin, project_id, owner, reviewer) = setup(&env);

    client
        .mock_all_auths()
        .set_pause_state(&admin, &PauseState::PausedAll, &None, &None);

    assert_eq!(client.get_pause_state(), PauseState::PausedAll);
    assert!(client.is_paused());

    assert_eq!(
        try_review(&client, project_id, &reviewer),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_eq!(
        try_verification(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_eq!(
        try_fee(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_eq!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_paused_verification_blocks_only_verification() {
    let env = Env::default();
    let (client, admin, project_id, owner, reviewer) = setup(&env);

    client.mock_all_auths().set_pause_state(
        &admin,
        &PauseState::PausedVerification,
        &None,
        &None,
    );

    assert_eq!(
        try_verification(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_ne!(
        try_review(&client, project_id, &reviewer),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_fee(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_paused_fees_blocks_only_fees() {
    let env = Env::default();
    let (client, admin, project_id, owner, reviewer) = setup(&env);

    client
        .mock_all_auths()
        .set_pause_state(&admin, &PauseState::PausedFees, &None, &None);

    assert_eq!(
        try_fee(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_ne!(
        try_review(&client, project_id, &reviewer),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_verification(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_paused_reviews_blocks_only_reviews() {
    let env = Env::default();
    let (client, admin, project_id, owner, reviewer) = setup(&env);

    client
        .mock_all_auths()
        .set_pause_state(&admin, &PauseState::PausedReviews, &None, &None);

    assert_eq!(
        try_review(&client, project_id, &reviewer),
        Err(Ok(ContractError::ContractPaused))
    );

    assert_ne!(
        try_verification(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_fee(&client, project_id, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
    assert_ne!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_contract_status_lists_blocked_operations() {
    let env = Env::default();
    let (client, admin, _project_id, _owner, _reviewer) = setup(&env);

    client
        .mock_all_auths()
        .set_pause_state(&admin, &PauseState::PausedReviews, &None, &None);

    let status = client.get_contract_status();
    assert!(status.is_paused);
    assert_eq!(status.pause_state, PauseState::PausedReviews);
    assert_eq!(status.paused_operations.len(), 1);
    assert_eq!(
        status.paused_operations.get(0),
        Some(PauseOperation::Reviews)
    );

    // Unpausing clears the blocked families again.
    client.mock_all_auths().unpause(&admin);
    let status = client.get_contract_status();
    assert!(!status.is_paused);
    assert_eq!(status.pause_state, PauseState::Running);
    assert!(status.paused_operations.is_empty());
}

#[test]
fn test_pause_state_requires_admin() {
    let env = Env::default();
    let (client, _admin, _project_id, _owner, _reviewer) = setup(&env);
    let stranger = Address::generate(&env);

    let result = client.mock_all_auths().try_set_pause_state(
        &stranger,
        &PauseState::PausedAll,
        &None,
        &None,
    );

    assert!(result.is_err(), "a non-admin must not be able to pause");
}

#[test]
fn test_unpause_restores_every_operation_family() {
    let env = Env::default();
    let (client, admin, _project_id, owner, _reviewer) = setup(&env);

    client
        .mock_all_auths()
        .set_pause_state(&admin, &PauseState::PausedAll, &None, &None);
    assert_eq!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );

    client.mock_all_auths().unpause(&admin);

    assert!(!client.is_paused());
    assert_ne!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}
