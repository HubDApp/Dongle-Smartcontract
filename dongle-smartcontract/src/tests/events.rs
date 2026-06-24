//! Event coverage for important state changes used by indexers.

use crate::events::{
    FeeConsumedEvent, FeeOperation, FeePaidEvent, FeeSetEvent, MinProjectAgeSetEvent,
    ProjectArchivedEvent, ProjectOwnershipTransferredEvent, ProjectReactivatedEvent,
    ReviewHiddenEvent, ReviewReportedEvent, ReviewRestoredEvent, ProjectReviewsEnabledSetEvent,
    ProjectClaimableSetEvent, ClaimRequestSubmittedEvent, ClaimRequestApprovedEvent,
    ClaimRequestRejectedEvent,
};
use crate::types::ProjectRegistrationParams;
use crate::{DongleContract, DongleContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    Address, Env, IntoVal, String, TryIntoVal, Val, Vec,
};

const TEST_TIMESTAMP: u64 = 1_700_000_123;

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    env.ledger().set(LedgerInfo {
        timestamp: TEST_TIMESTAMP,
        protocol_version: 22,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 4096,
        max_entry_ttl: 6_312_000,
    });

    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn register_project(
    client: &DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
) -> u64 {
    let slug = name.to_lowercase().replace(' ', "-");
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            slug: String::from_str(env, &slug),
            description: String::from_str(env, "Project description"),
            category: String::from_str(env, "DeFi"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
        })
}

fn decode_event<T: soroban_sdk::TryFromVal<Env, Val>>(env: &Env, data: &Val) -> Option<T> {
    TryIntoVal::<_, T>::try_into_val(data, env).ok()
}

fn has_event<T, Topics, F>(env: &Env, expected_topics: Topics, predicate: F) -> bool
where
    T: soroban_sdk::TryFromVal<Env, Val>,
    Topics: IntoVal<Env, Vec<Val>>,
    F: Fn(T) -> bool,
{
    let expected_topics = expected_topics.into_val(env);
    env.events().all().iter().any(|(_, topics, data)| {
        topics == expected_topics
            && decode_event::<T>(env, &data)
                .map(&predicate)
                .unwrap_or(false)
    })
}

fn setup_fee(
    env: &Env,
    client: &DongleContractClient<'_>,
    admin: &Address,
    owner: &Address,
    verification_fee: u128,
    registration_fee: u128,
) -> Address {
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    soroban_sdk::token::StellarAssetClient::new(env, &token).mint(owner, &10_000);
    client.mock_all_auths().set_fee(
        admin,
        &Some(token.clone()),
        &verification_fee,
        &registration_fee,
        admin,
    );
    token
}

#[test]
fn test_archive_and_reactivate_events_include_topics_and_payloads() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Archive-Me");

    client.mock_all_auths().archive_project(&project_id, &owner);
    assert!(has_event::<ProjectArchivedEvent, _, _>(
        &env,
        (
            symbol_short!("PROJECT"),
            symbol_short!("ARCHIVED"),
            project_id
        ),
        |event| {
            event.project_id == project_id
                && event.archived_by == owner
                && event.timestamp == TEST_TIMESTAMP
        }
    ));

    client.mock_all_auths().reactivate_project(&project_id, &owner);
    assert!(has_event::<ProjectReactivatedEvent, _, _>(
        &env,
        (
            symbol_short!("PROJECT"),
            symbol_short!("RESTORED"),
            project_id
        ),
        |event| {
            event.project_id == project_id
                && event.caller == owner
                && event.timestamp == TEST_TIMESTAMP
        }
    ));
}

#[test]
fn test_ownership_transfer_event_includes_topics_and_payload() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let old_owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &old_owner, "Transfer-Me");

    client
        .mock_all_auths()
        .initiate_transfer(&project_id, &old_owner, &new_owner);
    client
        .mock_all_auths()
        .accept_transfer(&project_id, &new_owner);

    assert!(has_event::<ProjectOwnershipTransferredEvent, _, _>(
        &env,
        (
            symbol_short!("PROJECT"),
            symbol_short!("TRANSFER"),
            project_id
        ),
        |event| {
            event.project_id == project_id
                && event.caller == new_owner
                && event.old_owner == old_owner
                && event.new_owner == new_owner
                && event.timestamp == TEST_TIMESTAMP
        }
    ));
}

#[test]
fn test_fee_paid_and_consumed_events_include_topics_and_payloads() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Fee-Project");
    let token = setup_fee(&env, &client, &admin, &owner, 200, 0);

    client
        .mock_all_auths()
        .pay_fee(&owner, &project_id, &Some(token.clone()));

    assert!(has_event::<FeePaidEvent, _, _>(
        &env,
        (symbol_short!("FEE"), symbol_short!("PAID"), project_id),
        |event| {
            event.project_id == project_id
                && event.payer == owner
                && event.token == Some(token.clone())
                && event.operation == FeeOperation::Verification
                && event.amount == 200
                && event.timestamp == TEST_TIMESTAMP
        }
    ));

    client.mock_all_auths().request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "QmEvidenceCid1234567890123456789012345678901234"),
    );

    assert!(has_event::<FeeConsumedEvent, _, _>(
        &env,
        (symbol_short!("FEE"), symbol_short!("CONSUMED"), project_id),
        |event| {
            event.project_id == project_id
                && event.caller == owner
                && event.operation == FeeOperation::Verification
                && event.amount == 200
                && event.timestamp == TEST_TIMESTAMP
        }
    ));
}

#[test]
fn test_review_moderation_events_include_topics_and_payloads() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let reporter = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Moderate-Me");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5, &None);
    client
        .mock_all_auths()
        .report_review(&project_id, &reviewer, &reporter);

    assert!(has_event::<ReviewReportedEvent, _, _>(
        &env,
        (
            symbol_short!("REVIEW"),
            symbol_short!("REPORTED"),
            project_id,
        ),
        |event| {
            event.project_id == project_id
                && event.reporter == reporter
                && event.reviewer == reviewer
                && event.timestamp == TEST_TIMESTAMP
        }
    ));

    client
        .mock_all_auths()
        .hide_review(&project_id, &reviewer, &admin);

    assert!(has_event::<ReviewHiddenEvent, _, _>(
        &env,
        (
            symbol_short!("REVIEW"),
            symbol_short!("HIDDEN"),
            project_id,
        ),
        |event| {
            event.project_id == project_id
                && event.admin == admin
                && event.reviewer == reviewer
                && event.timestamp == TEST_TIMESTAMP
        }
    ));

    client
        .mock_all_auths()
        .restore_review(&project_id, &reviewer, &admin);

    assert!(has_event::<ReviewRestoredEvent, _, _>(
        &env,
        (
            symbol_short!("REVIEW"),
            symbol_short!("RESTORED"),
            project_id,
        ),
        |event| {
            event.project_id == project_id
                && event.admin == admin
                && event.timestamp == TEST_TIMESTAMP
        }
    ));
}

#[test]
fn test_settings_events_include_topics_and_payloads() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    client
        .mock_all_auths()
        .set_fee(&admin, &Some(token.clone()), &300, &25, &owner);

    assert!(has_event::<FeeSetEvent, _, _>(
        &env,
        (symbol_short!("CONFIG"), symbol_short!("FEE")),
        |event| {
            event.admin == admin
                && event.token == Some(token.clone())
                && event.verification_fee == 300
                && event.registration_fee == 25
                && event.treasury == owner
                && event.timestamp == TEST_TIMESTAMP
        }
    ));

    client.mock_all_auths().set_min_project_age(&admin, &86_400);

    assert!(has_event::<MinProjectAgeSetEvent, _, _>(
        &env,
        (symbol_short!("CONFIG"), symbol_short!("MIN_AGE")),
        |event| {
            event.admin == admin
                && event.min_age_seconds == 86_400
                && event.timestamp == TEST_TIMESTAMP
        }
    ));
}

#[test]
fn test_project_reviews_enabled_set_event() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Review-Config-Project");

    // Disable reviews
    client.mock_all_auths().set_reviews_enabled(&project_id, &owner, &false);

    assert!(has_event::<ProjectReviewsEnabledSetEvent, _, _>(
        &env,
        (symbol_short!("PROJECT"), symbol_short!("REVIEWS"), project_id),
        |event| {
            event.project_id == project_id
                && event.caller == owner
                && event.enabled == false
                && event.timestamp == TEST_TIMESTAMP
        }
    ));
}

#[test]
fn test_project_claim_events() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "Claimable-Project");

    // 1. Set claimable to true
    client.mock_all_auths().set_project_claimable(&project_id, &owner, &true);

    assert!(has_event::<ProjectClaimableSetEvent, _, _>(
        &env,
        (symbol_short!("PROJECT"), symbol_short!("CLAIMABLE"), project_id),
        |event| {
            event.project_id == project_id
                && event.caller == owner
                && event.claimable == true
                && event.timestamp == TEST_TIMESTAMP
        }
    ));

    // 2. Submit claim request
    let proof_cid = String::from_str(&env, "QmTestProofCid");
    let claim_request_id = client.mock_all_auths().submit_claim_request(&project_id, &claimant, &proof_cid);

    assert!(has_event::<ClaimRequestSubmittedEvent, _, _>(
        &env,
        (
            symbol_short!("CLAIM"),
            symbol_short!("SUBMITTED"),
            project_id,
            claimant.clone()
        ),
        |event| {
            event.claim_request_id == claim_request_id
                && event.project_id == project_id
                && event.claimant == claimant
                && event.proof_cid == proof_cid
                && event.timestamp == TEST_TIMESTAMP
        }
    ));

    // 3. Reject claim request
    client.mock_all_auths().reject_claim_request(&claim_request_id, &admin);

    assert!(has_event::<ClaimRequestRejectedEvent, _, _>(
        &env,
        (
            symbol_short!("CLAIM"),
            symbol_short!("REJECTED"),
            project_id,
            claimant.clone()
        ),
        |event| {
            event.claim_request_id == claim_request_id
                && event.project_id == project_id
                && event.claimant == claimant
                && event.admin == admin
                && event.timestamp == TEST_TIMESTAMP
        }
    ));

    // 4. Submit claim request again (using claimant_2 since claimant already has a request record)
    let claimant_2 = Address::generate(&env);
    let claim_request_id_2 = client.mock_all_auths().submit_claim_request(&project_id, &claimant_2, &proof_cid);

    // 5. Approve claim request
    client.mock_all_auths().approve_claim_request(&claim_request_id_2, &admin);

    assert!(has_event::<ClaimRequestApprovedEvent, _, _>(
        &env,
        (
            symbol_short!("CLAIM"),
            symbol_short!("APPROVED"),
            project_id,
            claimant_2.clone()
        ),
        |event| {
            event.claim_request_id == claim_request_id_2
                && event.project_id == project_id
                && event.claimant == claimant_2
                && event.admin == admin
                && event.timestamp == TEST_TIMESTAMP
        }
    ));
}

