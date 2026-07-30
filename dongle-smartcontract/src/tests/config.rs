//! Tests for the contract-level `get_config` view + pause toggle.
//!
//! These tests pin the shape and semantics of the `ContractConfigView`
//! returned to frontends. Any breakage is a signal that the public
//! contract surface has changed and `CONTRACT_VERSION` likely needs a
//! bump.

#![allow(dead_code)]

use crate::constants::{
    CONTRACT_VERSION, MAX_DESCRIPTION_LEN, MAX_NAME_LEN, MAX_PAGE_LIMIT, MAX_PROJECTS_PER_USER,
    MAX_REVIEWS_PER_PROJECT, VERIFICATION_VALIDITY_PERIOD,
};
use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::{ContractConfigView, ContractLimits, FeeConfig};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

/// Helper: build a `ContractConfigView` with values that match the
/// expected initial state of a freshly-initialized contract (no fees,
/// one admin, paused=false). Pulls all values from `constants.rs` so
/// the test stays in sync if a default ever changes.
fn expected_initial(env: &Env) -> ContractConfigView {
    ContractConfigView {
        version: String::from_str(env, CONTRACT_VERSION),
        admin_count: 1,
        admin_approval_threshold: 1,
        paused: false,
        treasury: None,
        fees: FeeConfig {
            token: None,
            verification_fee: 0,
            registration_fee: 0,
        },
        limits: ContractLimits {
            max_page_limit: MAX_PAGE_LIMIT,
            max_projects_per_user: MAX_PROJECTS_PER_USER,
            max_reviews_per_project: MAX_REVIEWS_PER_PROJECT,
            max_name_len: MAX_NAME_LEN as u32,
            max_description_len: MAX_DESCRIPTION_LEN as u32,
            verification_validity_period: VERIFICATION_VALIDITY_PERIOD,
        },
    }
}

#[test]
fn test_get_config_initial_state_after_initialize() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let cfg = client.get_config();

    assert_eq!(cfg, expected_initial(&env));
}

#[test]
fn test_get_config_reflects_fee_update() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);

    client.mock_all_auths().initialize(&admin);

    // First fee set: verification only.
    client.mock_all_auths().set_fee(
        &admin,
        &Some(token.clone()),
        &1_000u128,
        &500u128,
        &treasury,
    );

    let cfg = client.get_config();
    assert_eq!(cfg.treasury, Some(treasury.clone()));
    assert_eq!(cfg.fees.verification_fee, 1_000);
    assert_eq!(cfg.fees.registration_fee, 500);
    assert_eq!(cfg.fees.token, Some(token.clone()));

    // Second fee set: token rotation + bumped amounts. The view must
    // reflect the latest values, not stale ones.
    let token2 = Address::generate(&env);
    let treasury2 = Address::generate(&env);
    client.mock_all_auths().set_fee(
        &admin,
        &Some(token2.clone()),
        &2_500u128,
        &1_250u128,
        &treasury2,
    );

    let cfg2 = client.get_config();
    assert_eq!(cfg2.treasury, Some(treasury2));
    assert_eq!(cfg2.fees.verification_fee, 2_500);
    assert_eq!(cfg2.fees.registration_fee, 1_250);
    assert_eq!(cfg2.fees.token, Some(token2));

    // Other fields stay constant across fee updates.
    assert_eq!(cfg2.version, cfg.version);
    assert_eq!(cfg2.limits, cfg.limits);
    assert_eq!(cfg2.admin_count, 1);
    assert_eq!(cfg2.paused, false);
}

#[test]
fn test_get_config_reflects_admin_count_change() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.mock_all_auths().initialize(&admin);
    client.mock_all_auths().set_fee(
        &admin,
        &None,
        &0u128,
        &0u128,
        &Address::generate(&env),
    );

    assert_eq!(client.get_config().admin_count, 1);

    client.mock_all_auths().add_admin(&admin, &new_admin);
    assert_eq!(client.get_config().admin_count, 2);

    client.mock_all_auths().remove_admin(&admin, &new_admin);
    assert_eq!(client.get_config().admin_count, 1);
}

#[test]
fn test_set_pause_admin_only() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);

    let result = client.mock_all_auths().try_set_pause(&non_admin, &true);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));

    // Admin call succeeds and returns the previous value (false here
    // because the flag has never been toggled). View still works
    // post-toggle since `get_config` defaults fees, so we re-assert
    // the *pause* change rather than the presence of an error.
    let prev = client.mock_all_auths().set_pause(&admin, &true);
    assert_eq!(prev, false);
    assert!(client.get_config().paused);
    assert_eq!(
        client.mock_all_auths().set_pause(&admin, &false),
        true,
        "previous flag should now report paused=true"
    );
    assert!(!client.get_config().paused);

    // Audit parity: set_pause records an AdminActionLog entry on every
    // transition with the matching ContractPaused/ContractResumed
    // variant. Verify both by reading the log back AND that the order
    // matches the call sequence — guards against future serialisation
    // bugs where both transitions accidentally land on the same variant.
    let log = client.list_admin_actions(&0u32, &10u32);
    let mut saw_paused = false;
    let mut saw_resumed_after_paused = false;
    for entry in log.iter() {
        match entry.action_type {
            crate::types::AdminActionType::ContractPaused => saw_paused = true,
            crate::types::AdminActionType::ContractResumed => {
                if saw_paused {
                    saw_resumed_after_paused = true;
                }
            }
            _ => {}
        }
    }
    assert!(saw_paused, "expected a ContractPaused log entry");
    assert!(
        saw_resumed_after_paused,
        "expected a ContractResumed entry AFTER ContractPaused"
    );
    let _ = admin; // explicit unused-binding marker
}

#[test]
fn test_get_config_reflects_pause_toggle() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.mock_all_auths().initialize(&admin);
    client.mock_all_auths().set_fee(
        &admin,
        &None,
        &0u128,
        &0u128,
        &Address::generate(&env),
    );

    let cfg = client.get_config();
    assert_eq!(cfg.paused, false);

    assert_eq!(client.mock_all_auths().set_pause(&admin, &true), false);
    assert_eq!(client.get_config().paused, true);

    assert_eq!(client.mock_all_auths().set_pause(&admin, &false), true);
    assert_eq!(client.get_config().paused, false);
}

#[test]
fn test_get_config_reflects_threshold_change() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    client.mock_all_auths().initialize(&admin);
    client.mock_all_auths().add_admin(&admin, &admin2);
    client.mock_all_auths().add_admin(&admin, &admin3);
    client.mock_all_auths().set_fee(
        &admin,
        &None,
        &0u128,
        &0u128,
        &Address::generate(&env),
    );

    // Default threshold is 1.
    assert_eq!(client.get_config().admin_approval_threshold, 1);

    // Bump to 2 and verify the new value is reflected.
    client.mock_all_auths().set_admin_approval_threshold(&admin, &2u32);
    assert_eq!(client.get_config().admin_approval_threshold, 2);

    // And back down to 1.
    client.mock_all_auths().set_admin_approval_threshold(&admin, &1u32);
    assert_eq!(client.get_config().admin_approval_threshold, 1);
}

/// Stable-shape guard: enumerates every field of `ContractConfigView`
/// and asserts each is non-default after a populated initial state. If
/// the struct grows accidentally between versions, callers can detect
/// it via ABI diffing — this test ensures at least the *current*
/// fields all participate in equality comparisons.
#[test]
fn test_get_config_shape_is_stable() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token = Address::generate(&env);

    client.mock_all_auths().initialize(&admin);
    client.mock_all_auths().set_fee(
        &admin,
        &Some(token.clone()),
        &42u128,
        &7u128,
        &treasury,
    );
    let cfg = client.get_config();

    // Each scalar is non-default after population — guards against
    // accidental field removal (which would still produce a "valid"
    // config otherwise).
    assert!(!cfg.version.is_empty());
    assert!(cfg.admin_count >= 1);
    assert!(cfg.admin_approval_threshold >= 1);
    assert!(cfg.treasury.is_some());
    assert!(cfg.fees.token.is_some());
    assert!(cfg.fees.verification_fee > 0);
    assert!(cfg.fees.registration_fee > 0);
    assert!(cfg.limits.max_page_limit > 0);
    assert!(cfg.limits.max_projects_per_user > 0);
    assert!(cfg.limits.max_reviews_per_project > 0);
    assert!(cfg.limits.max_name_len > 0);
    assert!(cfg.limits.max_description_len > 0);
    assert!(cfg.limits.verification_validity_period > 0);
}
