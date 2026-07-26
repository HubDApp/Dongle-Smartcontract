//! Tests for anti-sybil review eligibility constraints.
//!
//! Covers:
//! - Default config (fully permissive, backward compatible)
//! - Admin config management
//! - Minimum account age
//! - Endorsement requirement
//! - Fee requirement
//! - Combined constraints

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ReviewEligibilityConfig;
use crate::DongleContractClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env,
};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    setup_contract(env)
}

// ─── Default (permissive) ─────────────────────────────────────────────────

#[test]
fn test_default_config_is_permissive() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectA");

    let config = client.get_review_eligibility_config();
    assert_eq!(config.min_reviewer_age_seconds, 0);
    assert!(!config.require_endorsement);
    assert_eq!(config.review_fee, 0);

    // With default config, any address can review without restrictions.
    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&project_id, &reviewer, &4, &None);
    assert!(result.is_ok());
}

// ─── Admin config management ──────────────────────────────────────────────

#[test]
fn test_admin_can_set_eligibility_config() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 86400, // 1 day
        require_endorsement: true,
        review_fee: 100,
    };
    client.set_review_eligibility_config(&admin, &config);

    let stored = client.get_review_eligibility_config();
    assert_eq!(stored.min_reviewer_age_seconds, 86400);
    assert!(stored.require_endorsement);
    assert_eq!(stored.review_fee, 100);
}

#[test]
fn test_non_admin_cannot_set_eligibility_config() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let non_admin = Address::generate(&env);
    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 3600,
        require_endorsement: false,
        review_fee: 0,
    };

    // Use try_set_review_eligibility_config — but the client method name follows the contract.
    // Since mock_all_auths is enabled, auth passes but admin check will fail.
    // The generated client should have `try_set_review_eligibility_config`.
    let result = client.try_set_review_eligibility_config(&non_admin, &config);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly.into())));
}

#[test]
fn test_admin_can_restore_default_config() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    // Set a restrictive config
    let strict = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 3600,
        require_endorsement: true,
        review_fee: 100,
    };
    client.set_review_eligibility_config(&admin, &strict);

    // Restore to default
    let default = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 0,
        require_endorsement: false,
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &default);

    let stored = client.get_review_eligibility_config();
    assert_eq!(stored.min_reviewer_age_seconds, 0);
    assert!(!stored.require_endorsement);
    assert_eq!(stored.review_fee, 0);
}

// ─── Minimum account age ──────────────────────────────────────────────────

#[test]
fn test_min_reviewer_age_eligible() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "AgeEligible");

    // Set minimum age: 1000 seconds
    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 1000,
        require_endorsement: false,
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &config);

    let reviewer = Address::generate(&env);

    // Record first interaction 2000 seconds ago
    env.ledger().with_mut(|l| l.timestamp = 1000);
    client.add_review(&project_id, &reviewer, &3, &None);
    // The review was created, delete it so we can test re-review eligibility
    client.delete_review(&project_id, &reviewer);

    // Advance time past the minimum age threshold
    env.ledger().with_mut(|l| l.timestamp = 3000);

    // Now the reviewer should be eligible (2000s since first interaction > 1000s minimum)
    let result = client.try_add_review(&project_id, &reviewer, &4, &None);
    assert!(result.is_ok());
}

#[test]
fn test_min_reviewer_age_ineligible() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "AgeIneligible");

    // Set minimum age: 10000 seconds
    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 10000,
        require_endorsement: false,
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &config);

    let reviewer = Address::generate(&env);

    // Record first interaction at current time
    env.ledger().with_mut(|l| l.timestamp = 5000);

    // reviewer hasn't interacted yet — should be ineligible (no first_interaction)
    let result = client.try_add_review(&project_id, &reviewer, &3, &None);
    assert_eq!(result, Err(Ok(ContractError::ReviewerNotEligible.into())));
}

#[test]
fn test_min_reviewer_age_no_interaction_is_ineligible() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "NoInteraction");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 100,
        require_endorsement: false,
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &config);

    // This address has never interacted with the contract
    let stranger = Address::generate(&env);
    let result = client.try_add_review(&project_id, &stranger, &5, &None);
    assert_eq!(result, Err(Ok(ContractError::ReviewerNotEligible.into())));
}

#[test]
fn test_min_reviewer_age_zero_skips_check() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ZeroAge");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 0, // disabled
        require_endorsement: false,
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &config);

    // A brand-new address with no interaction should still be eligible
    let stranger = Address::generate(&env);
    let result = client.try_add_review(&project_id, &stranger, &5, &None);
    assert!(result.is_ok());
}

// ─── Endorsement requirement ──────────────────────────────────────────────

#[test]
fn test_require_endorsement_eligible() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "EndorseEligible");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 0,
        require_endorsement: true,
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &config);

    let reviewer = Address::generate(&env);

    // First, endorse the project
    client.endorse_project(&project_id, &reviewer);

    // Now the reviewer should be eligible to review
    let result = client.try_add_review(&project_id, &reviewer, &3, &None);
    assert!(result.is_ok());
}

#[test]
fn test_require_endorsement_ineligible() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "EndorseIneligible");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 0,
        require_endorsement: true,
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &config);

    // This reviewer has NOT endorsed the project
    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&project_id, &reviewer, &3, &None);
    assert_eq!(result, Err(Ok(ContractError::ReviewerNotEligible.into())));
}

#[test]
fn test_require_endorsement_false_skips_check() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "EndorseSkipped");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 0,
        require_endorsement: false, // disabled
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &config);

    // Reviewer has NOT endorsed, but requirement is off, so they should be eligible
    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&project_id, &reviewer, &4, &None);
    assert!(result.is_ok());
}

// ─── Fee requirement ──────────────────────────────────────────────────────

#[test]
fn test_review_fee_eligible() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "FeeEligible");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 0,
        require_endorsement: false,
        review_fee: 100,
    };
    client.set_review_eligibility_config(&admin, &config);

    let reviewer = Address::generate(&env);

    // For the review fee check we currently reuse FeeManager::is_fee_paid(project_id).
    // We need to pay the fee for the project. However pay_fee requires the project owner
    // to pay. Since mock_all_auths is on, we can call it through the project owner.
    let project = client.get_project(&project_id).unwrap();
    let owner = project.owner;

    // Pay the verification fee (this sets FeePaidForProject)
    // In the existing fee system pay_fee requires a token config. Since we don't have
    // a fee token configured, and the review fee check simply calls is_fee_paid(),
    // we can set the fee paid flag directly by calling pay_fee with token=None
    // if the fee config has verification_fee > 0.
    client.pay_fee(&owner, &project_id, &None);

    // Now the reviewer should be eligible (fee is paid for the project)
    let result = client.try_add_review(&project_id, &reviewer, &4, &None);
    assert!(result.is_ok());
}

#[test]
fn test_review_fee_ineligible() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "FeeIneligible");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 0,
        require_endorsement: false,
        review_fee: 100,
    };
    client.set_review_eligibility_config(&admin, &config);

    // Fee has NOT been paid for this project
    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&project_id, &reviewer, &4, &None);
    assert_eq!(result, Err(Ok(ContractError::ReviewFeeRequired.into())));
}

#[test]
fn test_review_fee_zero_skips_check() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "FeeZero");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 0,
        require_endorsement: false,
        review_fee: 0, // disabled
    };
    client.set_review_eligibility_config(&admin, &config);

    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&project_id, &reviewer, &3, &None);
    assert!(result.is_ok());
}

// ─── submit_review also respects eligibility ──────────────────────────────

#[test]
fn test_submit_review_respects_eligibility() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "SubmitEligible");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 99999, // unreachable without interaction
        require_endorsement: false,
        review_fee: 0,
    };
    client.set_review_eligibility_config(&admin, &config);

    let reviewer = Address::generate(&env);
    let cid = soroban_sdk::String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    // submit_review calls add_review which includes the eligibility check
    let result = client.try_submit_review(&project_id, &reviewer, &3, &cid);
    assert_eq!(result, Err(Ok(ContractError::ReviewerNotEligible.into())));
}

// ─── Combined constraints ─────────────────────────────────────────────────

#[test]
fn test_all_constraints_eligible() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "AllEligible");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 1000,
        require_endorsement: true,
        review_fee: 100,
    };
    client.set_review_eligibility_config(&admin, &config);

    let reviewer = Address::generate(&env);

    // 1. First interaction (age check)
    env.ledger().with_mut(|l| l.timestamp = 1000);
    client.add_review(&project_id, &reviewer, &2, &None);
    client.delete_review(&project_id, &reviewer);

    // 2. Endorse the project
    client.endorse_project(&project_id, &reviewer);

    // 3. Pay the fee
    let project = client.get_project(&project_id).unwrap();
    client.pay_fee(&project.owner, &project_id, &None);

    // 4. Advance time past the min age
    env.ledger().with_mut(|l| l.timestamp = 3000);

    // Now all constraints should pass
    let result = client.try_add_review(&project_id, &reviewer, &5, &None);
    assert!(result.is_ok());
}

#[test]
fn test_all_constraints_fail_if_one_missing() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "AllFailOneMissing");

    let config = ReviewEligibilityConfig {
        min_reviewer_age_seconds: 1000,
        require_endorsement: true,
        review_fee: 100,
    };
    client.set_review_eligibility_config(&admin, &config);

    let reviewer = Address::generate(&env);
    let project = client.get_project(&project_id).unwrap();

    // Only pay the fee — missing age and endorsement
    client.pay_fee(&project.owner, &project_id, &None);

    let result = client.try_add_review(&project_id, &reviewer, &4, &None);
    // Should fail with ReviewerNotEligible (first failing check is age)
    assert_eq!(result, Err(Ok(ContractError::ReviewerNotEligible.into())));
}

