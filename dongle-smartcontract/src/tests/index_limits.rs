//! Storage index size limit tests: owner projects and review indexes.

use crate::constants::{MAX_PROJECTS_PER_USER, MAX_REVIEWS_PER_PROJECT, MAX_REVIEWS_PER_USER};
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn register_project_for_owner(
    env: &Env,
    client: &crate::DongleContractClient<'_>,
    owner: &Address,
    name: &str,
) -> u64 {
    extern crate alloc;
    use alloc::format;
    let slug = name.to_lowercase().replace(' ', "-");
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "Test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    client.mock_all_auths().register_project(&params)
}

#[test]
fn test_max_projects_per_user_enforced() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 0..MAX_PROJECTS_PER_USER {
        extern crate alloc;
        use alloc::format;
        let name = format!("Project-{}", i);
        register_project_for_owner(&env, &client, &owner, &name);
    }

    assert_eq!(
        client.get_owner_project_count(&owner),
        MAX_PROJECTS_PER_USER
    );

    let result = client
        .mock_all_auths()
        .try_register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, "Overflow"),
            slug: String::from_str(&env, "overflow"),
            description: String::from_str(&env, "Too many projects"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
        });

    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}

#[test]
fn test_transfer_rejected_when_recipient_at_project_cap() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let extra_project = register_project_for_owner(&env, &client, &donor, "Donor-Project");

    for i in 0..MAX_PROJECTS_PER_USER {
        extern crate alloc;
        use alloc::format;
        let name = format!("Recipient-{}", i);
        register_project_for_owner(&env, &client, &recipient, &name);
    }

    client
        .mock_all_auths()
        .initiate_transfer(&extra_project, &donor, &recipient);

    let result = client
        .mock_all_auths()
        .try_accept_transfer(&extra_project, &recipient);

    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}

#[test]
fn test_claim_rejected_when_claimant_at_project_cap() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let project_id = register_project_for_owner(&env, &client, &owner, "Claimable-Project");

    for i in 0..MAX_PROJECTS_PER_USER {
        extern crate alloc;
        use alloc::format;
        let name = format!("Claimant-{}", i);
        register_project_for_owner(&env, &client, &claimant, &name);
    }

    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &owner, &true);
    let proof = String::from_str(&env, "QmClaimProofCid1234567890abcdef");
    let claim_id = client
        .mock_all_auths()
        .submit_claim_request(&project_id, &claimant, &proof);

    let result = client
        .mock_all_auths()
        .try_approve_claim_request(&claim_id, &admin);

    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}

#[test]
fn test_max_reviews_per_project_enforced() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "Review-Cap-Project");

    for i in 0..MAX_REVIEWS_PER_PROJECT {
        let reviewer = Address::generate(&env);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &3, &None);
        let _ = i;
    }

    let overflow_reviewer = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &overflow_reviewer, &4, &None);

    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}

#[test]
fn test_max_reviews_per_user_enforced() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let reviewer = Address::generate(&env);

    for i in 0..MAX_REVIEWS_PER_USER {
        extern crate alloc;
        use alloc::format;
        let owner = Address::generate(&env);
        let name = format!("Reviewed-{}", i);
        let project_id = register_project_for_owner(&env, &client, &owner, &name);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &5, &None);
    }

    let owner = Address::generate(&env);
    let overflow_project =
        register_project_for_owner(&env, &client, &owner, "Overflow-Review-Project");

    let result = client
        .mock_all_auths()
        .try_add_review(&overflow_project, &reviewer, &4, &None);

    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}
