//! Integration tests for input validation across all modules

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_env, register_test_project};
use crate::types::ProjectRegistrationParams;
use crate::DongleContract;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Project Registration Validation Tests ──

#[test]
fn test_register_project_with_valid_inputs() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Valid Project"),
        description: String::from_str(&env, "A valid project description"),
        category: String::from_str(&env, "DeFi"),
        website: Some(String::from_str(&env, "https://example.com")),
        logo_cid: Some(String::from_str(
            &env,
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
        )),
        metadata_cid: Some(String::from_str(
            &env,
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
        )),
    };

    let result = DongleContract::register_project(env.clone(), params);
    assert!(result.is_ok());
}

#[test]
#[should_panic(expected = "InvalidProjectName")]
fn test_register_project_empty_name() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, ""),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

#[test]
#[should_panic(expected = "InvalidProjectName")]
fn test_register_project_whitespace_only_name() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "   "),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

#[test]
#[should_panic(expected = "ProjectNameTooLong")]
fn test_register_project_name_too_long() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    // 51 characters
    let long_name = "ThisProjectNameIsWayTooLongAndExceedsTheFiftyCharL1";
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, long_name),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

#[test]
#[should_panic(expected = "InvalidProjectNameFormat")]
fn test_register_project_name_invalid_characters() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project@Name!"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

#[test]
#[should_panic(expected = "InvalidDescription")]
fn test_register_project_empty_description() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, ""),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

#[test]
#[should_panic(expected = "InvalidCategory")]
fn test_register_project_empty_category() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, "Valid description"),
        category: String::from_str(&env, ""),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

#[test]
#[should_panic(expected = "InvalidWebsiteUrlFormat")]
fn test_register_project_invalid_website_protocol() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, "Valid description"),
        category: String::from_str(&env, "DeFi"),
        website: Some(String::from_str(&env, "ftp://example.com")),
        logo_cid: None,
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

#[test]
#[should_panic(expected = "CidInvalidLength")]
fn test_register_project_invalid_cid_too_short() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, "Valid description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: Some(String::from_str(&env, "short")),
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

#[test]
#[should_panic(expected = "InvalidCidFormat")]
fn test_register_project_invalid_cid_format() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, "Valid description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: Some(String::from_str(&env, "Qm@#$%^&*()_+invalid")),
        metadata_cid: None,
    };

    let _ = DongleContract::register_project(env.clone(), params);
}

// ── Review Validation Tests ──

#[test]
fn test_add_review_with_valid_rating() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let result = DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(String::from_str(
            &env,
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
        )),
    );

    assert!(result.is_ok());
}

#[test]
fn test_add_review_rating_too_low() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let result = DongleContract::add_review(env.clone(), project_id, reviewer.clone(), 0, None);

    assert_eq!(result, Err(ContractError::InvalidRating));
}

#[test]
fn test_add_review_rating_too_high() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let result = DongleContract::add_review(env.clone(), project_id, reviewer.clone(), 6, None);

    assert_eq!(result, Err(ContractError::InvalidRating));
}

#[test]
fn test_add_review_invalid_comment_cid() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let result = DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(String::from_str(&env, "invalid@cid")),
    );

    assert_eq!(result, Err(ContractError::InvalidCidFormat));
}

// ── Pagination Validation Tests ──

#[test]
fn test_list_projects_with_valid_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let projects = DongleContract::list_projects(env.clone(), 1, 10);
    // Should not panic, returns empty vec if no projects
    assert_eq!(projects.len(), 0);
}

#[test]
fn test_list_projects_with_zero_limit() {
    let env = Env::default();
    env.mock_all_auths();

    // Should return empty vec due to validation failure
    let projects = DongleContract::list_projects(env.clone(), 1, 0);
    assert_eq!(projects.len(), 0);
}

#[test]
fn test_list_projects_with_excessive_limit() {
    let env = Env::default();
    env.mock_all_auths();

    // Should return empty vec due to validation failure (limit > 100)
    let projects = DongleContract::list_projects(env.clone(), 1, 101);
    assert_eq!(projects.len(), 0);
}

#[test]
fn test_list_reviews_with_valid_limit() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviews = DongleContract::list_reviews(env.clone(), project_id, 0, 10);
    assert_eq!(reviews.len(), 0);
}

#[test]
fn test_list_reviews_with_zero_limit() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviews = DongleContract::list_reviews(env.clone(), project_id, 0, 0);
    assert_eq!(reviews.len(), 0);
}

// ── Verification Evidence CID Tests ──

#[test]
#[should_panic(expected = "InvalidCid")]
fn test_request_verification_empty_evidence_cid() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let _ = DongleContract::request_verification(
        env.clone(),
        project_id,
        owner.clone(),
        String::from_str(&env, ""),
    );
}

#[test]
#[should_panic(expected = "CidInvalidLength")]
fn test_request_verification_evidence_cid_too_short() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let _ = DongleContract::request_verification(
        env.clone(),
        project_id,
        owner.clone(),
        String::from_str(&env, "short"),
    );
}

// ── Edge Case Tests ──

#[test]
fn test_project_name_at_max_length() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    // Exactly 50 characters
    let name = "12345678901234567890123456789012345678901234567890";
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, name),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::register_project(env.clone(), params);
    assert!(result.is_ok());
}

#[test]
fn test_all_valid_ratings() {
    let (env, admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    // Test all valid ratings (1-5)
    for rating in 1..=5 {
        let reviewer = Address::generate(&env);
        let result =
            DongleContract::add_review(env.clone(), project_id, reviewer.clone(), rating, None);
        assert!(result.is_ok(), "Rating {} should be valid", rating);
    }
}
