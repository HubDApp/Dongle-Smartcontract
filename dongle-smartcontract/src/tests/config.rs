use crate::constants::{
    MAX_PAGE_LIMIT, MAX_PROJECTS_PER_USER, MAX_REVIEWS_PER_PROJECT, VERIFICATION_VALIDITY_PERIOD,
};
use crate::tests::fixtures::setup_contract;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn config_after_initialization_exposes_defaults_and_limits() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let config = client.get_config();

    assert_eq!(config.admin_count, 1);
    assert!(!config.paused);
    assert_eq!(config.version, String::from_str(&env, "1.0.0"));
    assert_eq!(config.treasury, None);
    assert_eq!(config.fees.verification_fee, 0);
    assert_eq!(config.fees.registration_fee, 0);
    assert_eq!(config.limits.max_projects_per_user, MAX_PROJECTS_PER_USER);
    assert_eq!(config.limits.max_reviews_per_project, MAX_REVIEWS_PER_PROJECT);
    assert_eq!(config.limits.max_page_limit, MAX_PAGE_LIMIT);
    assert_eq!(
        config.limits.verification_validity_period,
        VERIFICATION_VALIDITY_PERIOD
    );
}

#[test]
fn config_reflects_fee_updates() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let treasury = Address::generate(&env);

    client.set_fee(&admin, &Some(token.clone()), &123u128, &45u128, &treasury);

    let config = client.get_config();

    assert_eq!(config.fees.token, Some(token));
    assert_eq!(config.fees.verification_fee, 123);
    assert_eq!(config.fees.registration_fee, 45);
    assert_eq!(config.treasury, Some(treasury));
    assert_eq!(config.admin_count, 1);
    assert!(!config.paused);
}
