//! Tests that validate event payloads emitted by the contract.
//!
//! Each test verifies that the correct event is emitted with all required
//! fields populated and correct values.

use crate::events::{
    AdminAddedEvent, AdminRemovedEvent, FeePaidEvent, FeeSetEvent, ProjectRegisteredEvent,
    ProjectUpdatedEvent, VerificationApprovedEvent, VerificationRejectedEvent,
    VerificationRequestedEvent,
};
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams, ReviewAction, ReviewEventData};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    Address, Env, String, TryIntoVal,
};

const TEST_TIMESTAMP: u64 = 1_700_000_000;

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

/// Setup with a non-zero ledger timestamp so timestamp fields are verifiable.
fn setup_with_timestamp(env: &Env) -> (DongleContractClient<'_>, Address) {
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
    setup(env)
}

fn make_project_params(env: &Env, owner: &Address, name: &str) -> ProjectRegistrationParams {
    ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        description: String::from_str(env, "A test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    }
}

/// Helper: decode event data into a typed struct, returning None on failure.
fn decode_event<T: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>(
    env: &Env,
    data: &soroban_sdk::Val,
) -> Option<T> {
    TryIntoVal::<_, T>::try_into_val(data, env).ok()
}

// ── Admin events ──────────────────────────────────────────────────────────────

#[test]
fn test_admin_added_event_on_initialize() {
    let env = Env::default();
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.mock_all_auths().initialize(&admin);

    let events = env.events().all();
    assert!(!events.is_empty(), "expected at least one event");

    let found = events.iter().any(|(_, _, data)| {
        decode_event::<AdminAddedEvent>(&env, &data)
            .map(|e| e.admin == admin)
            .unwrap_or(false)
    });
    assert!(found, "ADMIN ADDED event not found or has wrong payload");
}

#[test]
fn test_admin_added_event_fields() {
    let env = Env::default();
    let (client, admin1) = setup(&env);
    let admin2 = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin1, &admin2);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<AdminAddedEvent>(&env, &data)
            .map(|e| e.admin == admin2)
            .unwrap_or(false)
    });
    assert!(found, "ADMIN ADDED event missing for new admin");
}

#[test]
fn test_admin_added_event_has_timestamp() {
    let env = Env::default();
    let (client, admin1) = setup_with_timestamp(&env);
    let admin2 = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin1, &admin2);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<AdminAddedEvent>(&env, &data)
            .map(|e| e.admin == admin2 && e.timestamp == TEST_TIMESTAMP)
            .unwrap_or(false)
    });
    assert!(found, "ADMIN ADDED event missing timestamp field");
}

#[test]
fn test_admin_removed_event_fields() {
    let env = Env::default();
    let (client, admin1) = setup(&env);
    let admin2 = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin1, &admin2);
    client.mock_all_auths().remove_admin(&admin1, &admin2);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<AdminRemovedEvent>(&env, &data)
            .map(|e| e.admin == admin2)
            .unwrap_or(false)
    });
    assert!(
        found,
        "ADMIN REMOVED event missing or has wrong admin field"
    );
}

#[test]
fn test_admin_removed_event_has_timestamp() {
    let env = Env::default();
    let (client, admin1) = setup_with_timestamp(&env);
    let admin2 = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin1, &admin2);
    client.mock_all_auths().remove_admin(&admin1, &admin2);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<AdminRemovedEvent>(&env, &data)
            .map(|e| e.admin == admin2 && e.timestamp == TEST_TIMESTAMP)
            .unwrap_or(false)
    });
    assert!(found, "ADMIN REMOVED event missing timestamp field");
}

// ── Project events ────────────────────────────────────────────────────────────

#[test]
fn test_project_registered_event_fields() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "MyProject");
    let project_id = client.mock_all_auths().register_project(&params);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ProjectRegisteredEvent>(&env, &data)
            .map(|e| {
                e.project_id == project_id
                    && e.owner == owner
                    && e.name == String::from_str(&env, "MyProject")
                    && e.category == String::from_str(&env, "DeFi")
            })
            .unwrap_or(false)
    });
    assert!(
        found,
        "PROJECT CREATED event missing or has wrong payload fields"
    );
}

#[test]
fn test_project_registered_event_has_timestamp() {
    let env = Env::default();
    let (client, _admin) = setup_with_timestamp(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TimestampProject");
    let project_id = client.mock_all_auths().register_project(&params);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ProjectRegisteredEvent>(&env, &data)
            .map(|e| e.project_id == project_id && e.timestamp == TEST_TIMESTAMP)
            .unwrap_or(false)
    });
    assert!(found, "PROJECT CREATED event missing timestamp");
}

#[test]
fn test_project_updated_event_fields() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "UpdateMe");
    let project_id = client.mock_all_auths().register_project(&params);

    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "UpdatedName")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.mock_all_auths().update_project(&update_params);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ProjectUpdatedEvent>(&env, &data)
            .map(|e| e.project_id == project_id && e.owner == owner)
            .unwrap_or(false)
    });
    assert!(
        found,
        "PROJECT UPDATED event missing or has wrong payload fields"
    );
}

#[test]
fn test_project_updated_event_has_timestamp() {
    let env = Env::default();
    let (client, _admin) = setup_with_timestamp(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "UpdateTimestamp");
    let project_id = client.mock_all_auths().register_project(&params);

    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "NewName")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.mock_all_auths().update_project(&update_params);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ProjectUpdatedEvent>(&env, &data)
            .map(|e| e.project_id == project_id && e.timestamp == TEST_TIMESTAMP)
            .unwrap_or(false)
    });
    assert!(found, "PROJECT UPDATED event missing timestamp");
}

// ── Review events ─────────────────────────────────────────────────────────────

#[test]
fn test_review_submitted_event_fields() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let params = make_project_params(&env, &owner, "ReviewProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5, &None);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ReviewEventData>(&env, &data)
            .map(|e| {
                e.project_id == project_id
                    && e.reviewer == reviewer
                    && e.action == ReviewAction::Submitted
                    && e.created_at == e.updated_at
            })
            .unwrap_or(false)
    });
    assert!(
        found,
        "REVIEW SUBMITTED event missing or has wrong payload fields"
    );
}

#[test]
fn test_review_submitted_event_has_timestamp() {
    let env = Env::default();
    let (client, _admin) = setup_with_timestamp(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TimestampReview");
    let project_id = client.mock_all_auths().register_project(&params);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &4, &None);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ReviewEventData>(&env, &data)
            .map(|e| e.project_id == project_id && e.timestamp == TEST_TIMESTAMP)
            .unwrap_or(false)
    });
    assert!(found, "REVIEW SUBMITTED event missing timestamp");
}

#[test]
fn test_review_updated_event_fields() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let params = make_project_params(&env, &owner, "UpdateReview");
    let project_id = client.mock_all_auths().register_project(&params);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &3, &None);
    client
        .mock_all_auths()
        .update_review(&project_id, &reviewer, &5, &None);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ReviewEventData>(&env, &data)
            .map(|e| {
                e.project_id == project_id
                    && e.reviewer == reviewer
                    && e.action == ReviewAction::Updated
            })
            .unwrap_or(false)
    });
    assert!(
        found,
        "REVIEW UPDATED event missing or has wrong payload fields"
    );
}

#[test]
fn test_review_deleted_event_fields() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let params = make_project_params(&env, &owner, "DeleteReview");
    let project_id = client.mock_all_auths().register_project(&params);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &2, &None);
    client
        .mock_all_auths()
        .delete_review(&project_id, &reviewer);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ReviewEventData>(&env, &data)
            .map(|e| {
                e.project_id == project_id
                    && e.reviewer == reviewer
                    && e.action == ReviewAction::Deleted
            })
            .unwrap_or(false)
    });
    assert!(
        found,
        "REVIEW DELETED event missing or has wrong payload fields"
    );
}

#[test]
fn test_review_event_comment_cid_included() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let params = make_project_params(&env, &owner, "CommentReview");
    let project_id = client.mock_all_auths().register_project(&params);

    let cid = Some(String::from_str(&env, "QmTestCid123"));
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &4, &cid);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<ReviewEventData>(&env, &data)
            .map(|e| {
                e.project_id == project_id
                    && e.comment_cid == Some(String::from_str(&env, "QmTestCid123"))
            })
            .unwrap_or(false)
    });
    assert!(found, "REVIEW event missing comment_cid field");
}

// ── Fee events ────────────────────────────────────────────────────────────────

#[test]
fn test_fee_set_event_fields() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    client
        .mock_all_auths()
        .set_fee(&admin, &None, &500u128, &treasury);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<FeeSetEvent>(&env, &data)
            .map(|e| e.verification_fee == 500u128 && e.registration_fee == 0u128)
            .unwrap_or(false)
    });
    assert!(found, "FEE SET event missing or has wrong payload fields");
}

#[test]
fn test_fee_set_event_has_timestamp() {
    let env = Env::default();
    let (client, admin) = setup_with_timestamp(&env);
    let treasury = Address::generate(&env);

    client
        .mock_all_auths()
        .set_fee(&admin, &None, &100u128, &treasury);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<FeeSetEvent>(&env, &data)
            .map(|e| e.timestamp == TEST_TIMESTAMP)
            .unwrap_or(false)
    });
    assert!(found, "FEE SET event missing timestamp");
}

#[test]
fn test_fee_paid_event_fields() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "FeeProject");
    let project_id = client.register_project(&params);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);

    client.set_fee(&admin, &Some(token_address.clone()), &200u128, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<FeePaidEvent>(&env, &data)
            .map(|e| e.project_id == project_id && e.payer == owner && e.amount == 200u128)
            .unwrap_or(false)
    });
    assert!(
        found,
        "FEE PAID event missing or has wrong payload fields (project_id, payer, amount)"
    );
}

#[test]
fn test_fee_paid_event_has_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_with_timestamp(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "FeeTimestamp");
    let project_id = client.register_project(&params);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);

    client.set_fee(&admin, &Some(token_address.clone()), &100u128, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<FeePaidEvent>(&env, &data)
            .map(|e| e.project_id == project_id && e.timestamp == TEST_TIMESTAMP)
            .unwrap_or(false)
    });
    assert!(found, "FEE PAID event missing timestamp");
}

// ── Verification events ───────────────────────────────────────────────────────

#[test]
fn test_verification_requested_event_fields() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_with_timestamp(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "VerifyReq");
    let project_id = client.register_project(&params);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);

    client.set_fee(&admin, &Some(token_address.clone()), &100u128, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));

    let evidence = String::from_str(&env, "ipfs://evidence-cid");
    client.request_verification(&project_id, &owner, &evidence);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<VerificationRequestedEvent>(&env, &data)
            .map(|e| {
                e.project_id == project_id
                    && e.requester == owner
                    && e.evidence_cid == String::from_str(&env, "ipfs://evidence-cid")
                    && e.timestamp == TEST_TIMESTAMP
            })
            .unwrap_or(false)
    });
    assert!(
        found,
        "VERIFY REQ event missing or has wrong payload fields"
    );
}

#[test]
fn test_verification_approved_event_fields() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_with_timestamp(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "VerifyApp");
    let project_id = client.register_project(&params);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);

    client.set_fee(&admin, &Some(token_address.clone()), &100u128, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    client.approve_verification(&project_id, &admin);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<VerificationApprovedEvent>(&env, &data)
            .map(|e| {
                e.project_id == project_id && e.admin == admin && e.timestamp == TEST_TIMESTAMP
            })
            .unwrap_or(false)
    });
    assert!(
        found,
        "VERIFY APP event missing or has wrong payload fields"
    );
}

#[test]
fn test_verification_rejected_event_fields() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_with_timestamp(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "VerifyRej");
    let project_id = client.register_project(&params);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);

    client.set_fee(&admin, &Some(token_address.clone()), &100u128, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    client.reject_verification(&project_id, &admin);

    let events = env.events().all();
    let found = events.iter().any(|(_, _, data)| {
        decode_event::<VerificationRejectedEvent>(&env, &data)
            .map(|e| {
                e.project_id == project_id && e.admin == admin && e.timestamp == TEST_TIMESTAMP
            })
            .unwrap_or(false)
    });
    assert!(
        found,
        "VERIFY REJ event missing or has wrong payload fields"
    );
}
