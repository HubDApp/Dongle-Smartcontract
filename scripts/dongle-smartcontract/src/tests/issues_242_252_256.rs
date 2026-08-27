#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ContractClaimStatus, ProjectRegistrationParams, ProjectSortMode};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

#[test]
fn test_contract_address_claims() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Project-A");

    let contract_addr = String::from_str(
        &env,
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEWBE6YAAH2PBNU6K4R457OT7KMBM4",
    );
    let proof_cid = String::from_str(&env, "QmProofCID1234567890123456789012345678901234567");

    // 1. Claim contract
    let req = client.claim_contract_address(&project_id, &owner, &contract_addr, &proof_cid);
    assert_eq!(req.status, ContractClaimStatus::Pending);
    assert_eq!(req.contract_address, contract_addr);

    // 2. Reject claim
    client.reject_contract_claim(&project_id, &contract_addr, &admin);
    // Can't easily fetch request directly without getter, but let's re-claim and approve

    // We expect the next claim over the same address to just overwrite the rejected one
    let req2 = client.claim_contract_address(&project_id, &owner, &contract_addr, &proof_cid);
    assert_eq!(req2.status, ContractClaimStatus::Pending);

    // 3. Approve claim
    client.approve_contract_claim(&project_id, &contract_addr, &admin);

    // 4. Verify in get_verified_contracts
    let verified = client.get_verified_contracts(&project_id);
    assert_eq!(verified.len(), 1);
    assert_eq!(verified.get(0).unwrap(), contract_addr);
}

#[test]
fn test_project_sorting_options() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);

    // Create projects in order
    let p1 = create_test_project(&client, &owner1, "First-Project");

    let p2 = create_test_project(&client, &owner2, "Second-Project");

    // Submit reviews to affect rating and review count
    let reviewer = Address::generate(&env);
    client.submit_review(
        &p2,
        &reviewer,
        &5, // Rating 5
        &String::from_str(&env, "QmReview2........................................"),
    );

    // Sorting by MostReviewed -> p2 should be first
    let most_reviewed = client.list_projects_sorted(&ProjectSortMode::MostReviewed, &0, &10);
    assert_eq!(most_reviewed.get(0).unwrap().id, p2);
    assert_eq!(most_reviewed.get(1).unwrap().id, p1);

    // Sorting by HighestRated -> p2 should be first
    let highest_rated = client.list_projects_sorted(&ProjectSortMode::HighestRated, &0, &10);
    assert_eq!(highest_rated.get(0).unwrap().id, p2);
    assert_eq!(highest_rated.get(1).unwrap().id, p1);

    // Sorting by Oldest -> p1 should be first
    let oldest = client.list_projects_sorted(&ProjectSortMode::Oldest, &0, &10);
    assert_eq!(oldest.get(0).unwrap().id, p1);

    // Sorting by Newest
    let newest = client.list_projects_sorted(&ProjectSortMode::Newest, &0, &10);
    assert_eq!(newest.len(), 2);
}

#[test]
fn test_bounty_url_validation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);

    // Valid bounty URL
    let valid_bounty = String::from_str(&env, "https://immunefi.com/bounty/project");

    let params_valid = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Bounty-Project"),
        slug: String::from_str(&env, "bounty-project"),
        description: String::from_str(&env, "A project with a valid bug bounty URL..........."),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: Some(valid_bounty.clone()),
    };

    let proj_id = client.register_project(&params_valid);
    let proj = client.get_project(&proj_id).unwrap();
    assert_eq!(proj.bounty_url, Some(valid_bounty));

    // Invalid bounty URL - no protocol
    let invalid_bounty = String::from_str(&env, "invalid-url-without-http");
    let params_invalid = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Bounty-Project-2"),
        slug: String::from_str(&env, "bounty-project-2"),
        description: String::from_str(&env, "A project with an invalid bug bounty URL........"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: Some(invalid_bounty),
    };

    let res = client.try_register_project(&params_invalid);
    assert!(res.is_err(), "Should reject invalid bounty url");
}
