use crate::constants::{
    FEE_PAYMENT_EXPIRY_SECONDS, MAX_PAGE_LIMIT, MAX_PROJECTS_PER_USER, MAX_REVIEWS_PER_PROJECT,
    MAX_REVIEWS_PER_USER, MAX_SOCIAL_LINKS, MAX_TAGS_PER_PROJECT, REVIEW_UPDATE_COOLDOWN_SECONDS,
    VERIFICATION_VALIDITY_PERIOD,
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
    assert!(!config.has_fee_config);
    assert_eq!(config.treasury, None);
    assert_eq!(config.max_projects_per_user, MAX_PROJECTS_PER_USER);
    assert_eq!(config.max_reviews_per_project, MAX_REVIEWS_PER_PROJECT);
    assert_eq!(config.max_reviews_per_user, MAX_REVIEWS_PER_USER);
    assert_eq!(config.max_page_limit, MAX_PAGE_LIMIT);
    assert_eq!(config.max_tags_per_project, MAX_TAGS_PER_PROJECT);
    assert_eq!(config.max_social_links, MAX_SOCIAL_LINKS);
    assert_eq!(
        config.verification_validity_period,
        VERIFICATION_VALIDITY_PERIOD
    );
    assert_eq!(
        config.fee_payment_expiry_seconds,
        FEE_PAYMENT_EXPIRY_SECONDS
    );
    assert_eq!(
        config.review_update_cooldown_seconds,
        REVIEW_UPDATE_COOLDOWN_SECONDS
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

    assert!(config.has_fee_config);
    assert_eq!(config.fee_token, Some(token));
    assert_eq!(config.verification_fee, 123);
    assert_eq!(config.registration_fee, 45);
    assert_eq!(config.treasury, Some(treasury));
    assert_eq!(config.admin_count, 1);
    assert!(!config.paused);
}
