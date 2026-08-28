//! Integration test suite — multi-step, end-to-end workflows (issue #617).
//!
//! Unlike the per-module unit tests, every test here exercises a **sequence** of
//! public entry points the way a real client would drive them, and asserts on
//! the observable state after each step.
//!
//! Scenario index:
//!
//! Registration → verification → moderation
//!   1.  `registration_to_verification_approved`
//!   2.  `registration_to_verification_rejected`
//!   3.  `verification_then_revocation`
//!   4.  `verification_blocked_until_fee_paid`
//!   5.  `archive_then_reactivate_roundtrip`
//!   6.  `ownership_transfer_then_new_owner_updates`
//!
//! Reviews → moderation
//!   7.  `reviews_drive_project_stats`
//!   8.  `review_update_recomputes_average`
//!   9.  `report_then_hide_then_restore_review`
//!   10. `admin_delete_review_workflow`
//!   11. `disabling_reviews_blocks_new_reviews`
//!   12. `owner_responds_to_review`
//!
//! Admin governance
//!   13. `add_second_admin_then_remove_first`
//!   14. `cannot_remove_last_admin`
//!   15. `governance_proposal_add_admin_end_to_end`
//!   16. `pause_blocks_mutations_then_unpause_restores`
//!
//! Fee payment flows
//!   17. `verification_fee_paid_consumed_and_re_paid`
//!   18. `registration_fee_payment_flow`
//!   19. `fee_config_changes_are_recorded_in_history`
//!
//! Curation & social
//!   20. `collection_curation_workflow`
//!   21. `bookmark_endorse_follow_counters`

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, generate_test_users, setup_contract};
use crate::types::{ProjectRegistrationParams, ProposalPayload, VerificationStatus};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// A valid IPFS CIDv0, reused wherever a real CID is required.
const CID: &str = "QmTu64kW8cUwwigCcJcKQS6F6wTwwJeD8Y18qr9s9DXkXy";

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

/// Register a fee token, set the verification fee to 100 (registration free),
/// mint `mint_amount` to `payer` and return the token address.
fn configure_fee_token(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    admin: &Address,
    treasury: &Address,
    verification_fee: u128,
    registration_fee: u128,
    payer: &Address,
    mint_amount: i128,
) -> Address {
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.set_fee(
        admin,
        &Some(token.clone()),
        &verification_fee,
        &registration_fee,
        treasury,
    );
    soroban_sdk::token::StellarAssetClient::new(env, &token).mint(payer, &mint_amount);
    token
}

// ───────────────────────────────────────────────────────────────────────────
// 1–6: registration → verification → lifecycle
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn registration_to_verification_approved() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = create_test_project(&client, &owner, "approve-flow");
    assert_eq!(
        client.get_project(&id).unwrap().verification_status,
        VerificationStatus::Unverified
    );

    client.request_verification(&id, &owner, &s(&env, CID));
    assert_eq!(
        client.get_project(&id).unwrap().verification_status,
        VerificationStatus::Pending
    );

    client.approve_verification(&id, &admin);
    assert_eq!(
        client.get_project(&id).unwrap().verification_status,
        VerificationStatus::Verified
    );
    assert!(client.is_verification_active(&id));
}

#[test]
fn registration_to_verification_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = create_test_project(&client, &owner, "reject-flow");
    client.request_verification(&id, &owner, &s(&env, CID));
    client.reject_verification(&id, &admin);

    let status = client.get_project(&id).unwrap().verification_status;
    assert!(
        status == VerificationStatus::Unverified || status == VerificationStatus::Rejected,
        "after rejection the project must not be Verified/Pending, got {:?}",
        status
    );
    assert!(!client.is_verification_active(&id));
}

#[test]
fn verification_then_revocation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = create_test_project(&client, &owner, "revoke-flow");
    client.request_verification(&id, &owner, &s(&env, CID));
    client.approve_verification(&id, &admin);
    assert!(client.is_verification_active(&id));

    client.revoke_verification(&id, &admin, &s(&env, "policy violation"));
    assert_eq!(
        client.get_project(&id).unwrap().verification_status,
        VerificationStatus::Unverified
    );
    assert!(!client.is_verification_active(&id));
}

#[test]
fn verification_blocked_until_fee_paid() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let treasury = Address::generate(&env);
    let owner = Address::generate(&env);

    let id = create_test_project(&client, &owner, "fee-gate-flow");
    let token = configure_fee_token(&client, &env, &admin, &treasury, 100, 0, &owner, 500);

    // No payment yet → rejected.
    let err = client
        .try_request_verification(&id, &owner, &s(&env, CID))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::InsufficientFee);

    // Pay, then it succeeds.
    client.pay_fee(&owner, &id, &Some(token));
    assert!(client.is_fee_paid(&id));
    client.request_verification(&id, &owner, &s(&env, CID));
    assert_eq!(
        client.get_project(&id).unwrap().verification_status,
        VerificationStatus::Pending
    );
    // Fee consumed.
    assert!(!client.is_fee_paid(&id));
}

#[test]
fn archive_then_reactivate_roundtrip() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = create_test_project(&client, &owner, "archive-flow");
    assert!(!client.get_project(&id).unwrap().archived);

    client.archive_project(&id, &owner);
    assert!(client.get_project(&id).unwrap().archived);

    // Double archive is rejected.
    let err = client.try_archive_project(&id, &owner).unwrap_err().unwrap();
    assert_eq!(err, ContractError::AlreadyArchived);

    client.reactivate_project(&id, &owner);
    assert!(!client.get_project(&id).unwrap().archived);
}

#[test]
fn ownership_transfer_then_new_owner_updates() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);

    let id = create_test_project(&client, &owner, "transfer-flow");

    client.initiate_transfer(&id, &owner, &new_owner);
    // Still the old owner until accepted.
    assert_eq!(client.get_project(&id).unwrap().owner, owner);

    client.accept_transfer(&id, &new_owner);
    assert_eq!(client.get_project(&id).unwrap().owner, new_owner);

    // New owner can now drive owner-only operations.
    client.set_reviews_enabled(&id, &new_owner, &false);
    assert!(!client.get_reviews_enabled(&id));

    // Old owner can no longer.
    let err = client
        .try_set_reviews_enabled(&id, &owner, &true)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized);
}

// ───────────────────────────────────────────────────────────────────────────
// 7–12: reviews & moderation
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn reviews_drive_project_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "stats-flow");

    let reviewers = generate_test_users(&env, 3);
    client.add_review(&id, &reviewers.get(0).unwrap(), &5u32, &None);
    client.add_review(&id, &reviewers.get(1).unwrap(), &3u32, &None);
    client.add_review(&id, &reviewers.get(2).unwrap(), &4u32, &None);

    let stats = client.get_project_stats(&id);
    assert_eq!(stats.review_count, 3);
    // rating_sum and average_rating are both scaled ×100 (see RatingCalculator).
    assert_eq!(stats.rating_sum, 1_200);
    assert_eq!(stats.average_rating, 400); // (5 + 3 + 4) / 3 = 4.00
}

#[test]
fn review_update_recomputes_average() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "update-avg-flow");
    let reviewer = Address::generate(&env);

    client.add_review(&id, &reviewer, &2u32, &None);
    assert_eq!(client.get_project_stats(&id).average_rating, 200);

    client.update_review(&id, &reviewer, &5u32, &None);
    let stats = client.get_project_stats(&id);
    assert_eq!(stats.review_count, 1, "update must not add a second review");
    assert_eq!(stats.average_rating, 500);
}

#[test]
fn report_then_hide_then_restore_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "moderation-flow");
    let reviewer = Address::generate(&env);
    let reporter = Address::generate(&env);

    client.add_review(&id, &reviewer, &1u32, &None);
    client.report_review(&id, &reviewer, &reporter);

    // Reporting twice from the same address is rejected.
    let err = client
        .try_report_review(&id, &reviewer, &reporter)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::AlreadyReported);

    client.hide_review(&id, &reviewer, &admin);
    assert!(client.get_review(&id, &reviewer).unwrap().hidden);

    // Hiding an already-hidden review is rejected.
    let err = client
        .try_hide_review(&id, &reviewer, &admin)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::ReviewAlreadyHidden);

    client.restore_review(&id, &reviewer, &admin);
    assert!(!client.get_review(&id, &reviewer).unwrap().hidden);
}

#[test]
fn admin_delete_review_workflow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "admin-del-flow");
    let reviewer = Address::generate(&env);

    client.add_review(&id, &reviewer, &4u32, &None);
    assert_eq!(client.get_project_stats(&id).review_count, 1);

    client.admin_delete_review(&id, &reviewer, &admin);
    assert!(client.get_review(&id, &reviewer).is_none());
    assert_eq!(client.get_project_stats(&id).review_count, 0);
}

#[test]
fn disabling_reviews_blocks_new_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "reviews-off-flow");
    let reviewer = Address::generate(&env);

    client.set_reviews_enabled(&id, &owner, &false);
    let err = client
        .try_add_review(&id, &reviewer, &5u32, &None)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::ReviewsDisabled);

    client.set_reviews_enabled(&id, &owner, &true);
    client.add_review(&id, &reviewer, &5u32, &None);
    assert_eq!(client.get_project_stats(&id).review_count, 1);
}

#[test]
fn owner_responds_to_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "response-flow");
    let reviewer = Address::generate(&env);

    client.add_review(&id, &reviewer, &2u32, &None);
    client.respond_to_review(&id, &owner, &reviewer, &s(&env, "Thanks, fixed in v2"));

    assert_eq!(
        client.get_review_response(&id, &reviewer).unwrap(),
        s(&env, "Thanks, fixed in v2")
    );
}

// ───────────────────────────────────────────────────────────────────────────
// 13–16: admin governance
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn add_second_admin_then_remove_first() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let admin2 = Address::generate(&env);

    client.add_admin(&admin, &admin2);
    assert!(client.is_admin(&admin2));
    assert_eq!(client.get_admin_count(), 2);

    client.remove_admin(&admin2, &admin);
    assert!(!client.is_admin(&admin));
    assert_eq!(client.get_admin_count(), 1);
}

#[test]
fn cannot_remove_last_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let err = client.try_remove_admin(&admin, &admin).unwrap_err().unwrap();
    assert_eq!(err, ContractError::CannotRemoveLastAdmin);
    assert_eq!(client.get_admin_count(), 1);
}

#[test]
fn governance_proposal_add_admin_end_to_end() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let candidate = Address::generate(&env);

    // Threshold defaults to 1 → proposer's auto-approval is enough to execute.
    let proposal_id = client.create_proposal(
        &admin,
        &ProposalPayload::AddAdmin(candidate.clone()),
        &0u64,
    );

    assert!(client.get_proposal(&proposal_id).is_some());

    // Ensure the proposer's approval is recorded (no-op if create_proposal
    // already auto-approved it).
    let _ = client.try_approve_proposal(&admin, &proposal_id);

    client.execute_proposal(&admin, &proposal_id);
    assert!(client.is_admin(&candidate));
}

#[test]
fn pause_blocks_mutations_then_unpause_restores() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // Create a project before pausing.
    let id = create_test_project(&client, &owner, "pause-flow");

    client.pause(&admin);
    assert!(client.is_paused());

    // Registering a new project while paused is rejected.
    let owner2 = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner2.clone(),
        name: s(&env, "pause-flow-2"),
        slug: s(&env, "pause-flow-2"),
        description: s(&env, "Test project description"),
        category: s(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    };
    let err = client.try_register_project(&params).unwrap_err().unwrap();
    assert_eq!(err, ContractError::ContractPaused);

    client.unpause(&admin);
    assert!(!client.is_paused());

    // Now registration works again.
    let id2 = client.register_project(&params);
    assert_ne!(id, id2);
}

// ───────────────────────────────────────────────────────────────────────────
// 17–19: fee payment flows
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn verification_fee_paid_consumed_and_re_paid() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let treasury = Address::generate(&env);
    let owner = Address::generate(&env);

    let id = create_test_project(&client, &owner, "fee-cycle-flow");
    let token = configure_fee_token(&client, &env, &admin, &treasury, 100, 0, &owner, 300);
    let token_client = soroban_sdk::token::Client::new(&env, &token);

    // Pay #1 → consume via request_verification.
    client.pay_fee(&owner, &id, &Some(token.clone()));
    assert_eq!(token_client.balance(&treasury), 100);
    client.request_verification(&id, &owner, &s(&env, CID));
    assert!(!client.is_fee_paid(&id));

    // Approve then revoke to return to a state that allows a fresh request.
    client.approve_verification(&id, &admin);
    client.revoke_verification(&id, &admin, &s(&env, "re-pay test"));

    // Second request without paying again → rejected.
    let err = client
        .try_request_verification(&id, &owner, &s(&env, CID))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::InsufficientFee);

    // Re-pay → succeeds.
    client.pay_fee(&owner, &id, &Some(token));
    client.request_verification(&id, &owner, &s(&env, CID));
    assert_eq!(token_client.balance(&treasury), 200);
}

#[test]
fn registration_fee_payment_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let treasury = Address::generate(&env);
    let owner = Address::generate(&env);

    // registration_fee = 50, verification_fee = 0.
    let token = configure_fee_token(&client, &env, &admin, &treasury, 0, 50, &owner, 200);
    let token_client = soroban_sdk::token::Client::new(&env, &token);

    client.pay_registration_fee(&owner, &Some(token.clone()));
    assert_eq!(token_client.balance(&treasury), 50);

    let rec = client.get_reg_fee_payment_details(&owner).unwrap();
    assert_eq!(rec.payer, owner);
    assert_eq!(rec.amount, 50u128);
}

#[test]
fn fee_config_changes_are_recorded_in_history() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let treasury = Address::generate(&env);

    client.set_fee(&admin, &None, &10u128, &0u128, &treasury);
    client.set_fee(&admin, &None, &20u128, &5u128, &treasury);
    client.set_fee(&admin, &None, &30u128, &0u128, &treasury);

    let history = client.get_fee_config_history();
    assert!(
        history.len() >= 3,
        "each set_fee must append a history entry, got {}",
        history.len()
    );
    let cfg = client.get_fee_config();
    assert_eq!(cfg.verification_fee, 30u128);
}

// ───────────────────────────────────────────────────────────────────────────
// 20–21: curation & social
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn collection_curation_workflow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "coll-a");
    let p2 = create_test_project(&client, &owner, "coll-b");

    let coll = client.create_collection(&admin, &s(&env, "Editors Picks"), &s(&env, "curated"));

    client.add_project_to_collection(&admin, &coll, &p1);
    client.add_project_to_collection(&admin, &coll, &p2);
    assert_eq!(client.get_collection_project_count(&coll), 2);

    // Adding the same project twice is rejected.
    let err = client
        .try_add_project_to_collection(&admin, &coll, &p1)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::AlreadyInCollection);

    client.remove_project_from_collection(&admin, &coll, &p1);
    assert_eq!(client.get_collection_project_count(&coll), 1);

    // Removing a project that is not in the collection is rejected.
    let err = client
        .try_remove_project_from_collection(&admin, &coll, &p1)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::NotInCollection);
}

#[test]
fn bookmark_endorse_follow_counters() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "social-flow");
    let user = Address::generate(&env);

    client.bookmark_project(&id, &user);
    assert!(client.is_bookmarked(&id, &user));

    client.endorse_project(&id, &user);
    assert_eq!(client.get_endorsement_count(&id), 1);

    client.follow_project(&id, &user);
    assert_eq!(client.get_follower_count(&id), 1);

    // Idempotency guards.
    assert_eq!(
        client
            .try_bookmark_project(&id, &user)
            .unwrap_err()
            .unwrap(),
        ContractError::AlreadyBookmarked
    );
    assert_eq!(
        client
            .try_endorse_project(&id, &user)
            .unwrap_err()
            .unwrap(),
        ContractError::AlreadyEndorsed
    );

    client.unendorse_project(&id, &user);
    assert_eq!(client.get_endorsement_count(&id), 0);
}
