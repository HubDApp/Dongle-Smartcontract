//! Tests for community voting on collection membership — issue #907.

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::{ProjectRegistrationParams, VoteDirection};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn register_project(env: &Env, client: &crate::DongleContractClient<'_>, owner: &Address) -> u64 {
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, "Votable Project"),
        slug: String::from_str(env, "votable-project"),
        description: String::from_str(env, "Project up for a membership vote"),
        category: String::from_str(env, "DeFi"),
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
    };
    client.mock_all_auths().register_project(&params)
}

/// A voter needs a track record: one authored review is enough.
fn make_reputable_voter(
    env: &Env,
    client: &crate::DongleContractClient<'_>,
    project_id: u64,
) -> Address {
    let voter = Address::generate(env);
    client
        .mock_all_auths()
        .add_review(&project_id, &voter, &5u32, &None);
    voter
}

#[test]
fn test_voting_disabled_by_default() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&env, &client, &owner);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );

    let voter = make_reputable_voter(&env, &client, project_id);
    let result = client.mock_all_auths().try_vote_on_collection_membership(
        &voter,
        &collection_id,
        &project_id,
        &VoteDirection::Approve,
    );

    assert_eq!(result, Err(Ok(ContractError::CollectionVotingDisabled)));
}

#[test]
fn test_duplicate_vote_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&env, &client, &owner);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );
    client
        .mock_all_auths()
        .set_collection_voting(&admin, &collection_id, &true, &7500);

    let voter = make_reputable_voter(&env, &client, project_id);
    client.mock_all_auths().vote_on_collection_membership(
        &voter,
        &collection_id,
        &project_id,
        &VoteDirection::Approve,
    );

    let result = client.mock_all_auths().try_vote_on_collection_membership(
        &voter,
        &collection_id,
        &project_id,
        &VoteDirection::Approve,
    );
    assert_eq!(result, Err(Ok(ContractError::CollectionVoteAlreadyCast)));
}

#[test]
fn test_voter_without_reputation_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&env, &client, &owner);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );
    client
        .mock_all_auths()
        .set_collection_voting(&admin, &collection_id, &true, &7500);

    let newcomer = Address::generate(&env);
    let result = client.mock_all_auths().try_vote_on_collection_membership(
        &newcomer,
        &collection_id,
        &project_id,
        &VoteDirection::Approve,
    );

    assert_eq!(
        result,
        Err(Ok(ContractError::CollectionVoteInsufficientReputation))
    );
}

#[test]
fn test_project_owner_cannot_vote_for_own_project() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&env, &client, &owner);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );
    client
        .mock_all_auths()
        .set_collection_voting(&admin, &collection_id, &true, &7500);

    let result = client.mock_all_auths().try_vote_on_collection_membership(
        &owner,
        &collection_id,
        &project_id,
        &VoteDirection::Approve,
    );

    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_threshold_auto_accepts_project() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&env, &client, &owner);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );
    client
        .mock_all_auths()
        .set_collection_voting(&admin, &collection_id, &true, &7500);

    // Three approving votes cross the 75 % threshold (quorum is 3).
    let mut voters: Vec<Address> = Vec::new(&env);
    for _ in 0..3 {
        let voter = make_reputable_voter(&env, &client, project_id);
        let tally = client.mock_all_auths().vote_on_collection_membership(
            &voter,
            &collection_id,
            &project_id,
            &VoteDirection::Approve,
        );
        voters.push_back(voter);
        if voters.len() < 3 {
            assert_eq!(tally.outcome, crate::types::MembershipVoteOutcome::Pending);
        } else {
            assert_eq!(tally.outcome, crate::types::MembershipVoteOutcome::Approved);
            assert!(tally.resolved);
        }
    }

    // The project is now a member of the collection.
    let members = client.list_collection_projects(&collection_id);
    assert_eq!(members.len(), 1);
    assert_eq!(members.get(0), Some(project_id));

    let tally = client.get_collection_membership_votes(&collection_id, &project_id);
    assert_eq!(tally.approve_votes, 3);
    assert_eq!(tally.reject_votes, 0);
    assert_eq!(tally.approval_bps, 10_000);
    assert_eq!(tally.threshold_bps, 7500);
}

#[test]
fn test_rejection_threshold_removes_project() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&env, &client, &owner);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );
    client
        .mock_all_auths()
        .set_collection_voting(&admin, &collection_id, &true, &7500);
    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &collection_id, &project_id);

    for _ in 0..3 {
        let voter = make_reputable_voter(&env, &client, project_id);
        client.mock_all_auths().vote_on_collection_membership(
            &voter,
            &collection_id,
            &project_id,
            &VoteDirection::Reject,
        );
    }

    let members = client.list_collection_projects(&collection_id);
    assert!(members.iter().all(|id| id != project_id));

    let tally = client.get_collection_membership_votes(&collection_id, &project_id);
    assert_eq!(tally.reject_votes, 3);
    assert_eq!(tally.outcome, crate::types::MembershipVoteOutcome::Rejected);
}
