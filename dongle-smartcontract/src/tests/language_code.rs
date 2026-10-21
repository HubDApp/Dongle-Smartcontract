#![cfg(test)]

use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::types::{Project, ProjectRegistrationParams, ProjectUpdateParams, Review};
use crate::utils::Utils;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Map, String as SorobanString, Vec as SorobanVec};

fn create_env() -> Env {
    Env::default()
}

// ── Language Code Validation Tests ──

#[test]
fn test_validate_language_code_valid_codes() {
    let env = create_env();
    
    // Valid ISO 639-1 codes
    let valid_codes = vec!["en", "es", "fr", "de", "zh", "ja", "ko", "ru", "ar", "pt"];
    
    for code_str in valid_codes {
        let code = SorobanString::from_str(&env, code_str);
        assert!(Utils::validate_language_code(&code).is_ok(), 
            "Code {} should be valid", code_str);
    }
}

#[test]
fn test_validate_language_code_invalid_length() {
    let env = create_env();
    
    // Too short
    let code = SorobanString::from_str(&env, "e");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
    
    // Too long
    let code = SorobanString::from_str(&env, "eng");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
    
    // Empty
    let code = SorobanString::from_str(&env, "");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
}

#[test]
fn test_validate_language_code_invalid_characters() {
    let env = create_env();
    
    // Uppercase
    let code = SorobanString::from_str(&env, "EN");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
    
    // Mixed case
    let code = SorobanString::from_str(&env, "En");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
    
    // Numbers
    let code = SorobanString::from_str(&env, "e1");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
    
    // Special characters
    let code = SorobanString::from_str(&env, "e-");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
    
    let code = SorobanString::from_str(&env, "e_");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
    
    let code = SorobanString::from_str(&env, "e ");
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
}

// ── Project Language Code Tests ──

#[test]
fn test_register_project_with_valid_language_code() {
    let env = create_env();
    env.mock_all_auths();
    
    let owner = Address::generate(&env);
    
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: SorobanString::from_str(&env, "Test Project"),
        slug: SorobanString::from_str(&env, "test-project"),
        description: SorobanString::from_str(&env, "A test project with language code"),
        category: SorobanString::from_str(&env, "DeFi"),
        website: Some(SorobanString::from_str(&env, "https://test.com")),
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: Some(SorobanString::from_str(&env, "en")),
    };
    
    let result = ProjectRegistry::register_project(&env, params);
    assert!(result.is_ok());
    
    let project_id = result.unwrap();
    let project = ProjectRegistry::get_project(&env, project_id).unwrap();
    assert_eq!(project.language_code, Some(SorobanString::from_str(&env, "en")));
}

#[test]
fn test_register_project_without_language_code() {
    let env = create_env();
    env.mock_all_auths();
    
    let owner = Address::generate(&env);
    
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: SorobanString::from_str(&env, "Test Project"),
        slug: SorobanString::from_str(&env, "test-project"),
        description: SorobanString::from_str(&env, "A test project without language code"),
        category: SorobanString::from_str(&env, "DeFi"),
        website: Some(SorobanString::from_str(&env, "https://test.com")),
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
    
    let result = ProjectRegistry::register_project(&env, params);
    assert!(result.is_ok());
    
    let project_id = result.unwrap();
    let project = ProjectRegistry::get_project(&env, project_id).unwrap();
    assert_eq!(project.language_code, None);
}

#[test]
fn test_register_project_with_invalid_language_code() {
    let env = create_env();
    env.mock_all_auths();
    
    let owner = Address::generate(&env);
    
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: SorobanString::from_str(&env, "Test Project"),
        slug: SorobanString::from_str(&env, "test-project"),
        description: SorobanString::from_str(&env, "A test project with invalid language code"),
        category: SorobanString::from_str(&env, "DeFi"),
        website: Some(SorobanString::from_str(&env, "https://test.com")),
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: Some(SorobanString::from_str(&env, "ENG")), // Invalid: uppercase and 3 chars
    };
    
    let result = ProjectRegistry::register_project(&env, params);
    assert_eq!(result, Err(ContractError::InvalidLanguageCode));
}

#[test]
fn test_update_project_language_code() {
    let env = create_env();
    env.mock_all_auths();
    
    let owner = Address::generate(&env);
    
    // Register without language code
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: SorobanString::from_str(&env, "Test Project"),
        slug: SorobanString::from_str(&env, "test-project"),
        description: SorobanString::from_str(&env, "A test project"),
        category: SorobanString::from_str(&env, "DeFi"),
        website: Some(SorobanString::from_str(&env, "https://test.com")),
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
    
    let project_id = ProjectRegistry::register_project(&env, params).unwrap();
    
    // Update to add language code
    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: Some(Some(SorobanString::from_str(&env, "es"))),
    };
    
    let result = ProjectRegistry::update_project(&env, update_params);
    assert!(result.is_ok());
    
    let project = ProjectRegistry::get_project(&env, project_id).unwrap();
    assert_eq!(project.language_code, Some(SorobanString::from_str(&env, "es")));
}

#[test]
fn test_update_project_clear_language_code() {
    let env = create_env();
    env.mock_all_auths();
    
    let owner = Address::generate(&env);
    
    // Register with language code
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: SorobanString::from_str(&env, "Test Project"),
        slug: SorobanString::from_str(&env, "test-project"),
        description: SorobanString::from_str(&env, "A test project"),
        category: SorobanString::from_str(&env, "DeFi"),
        website: Some(SorobanString::from_str(&env, "https://test.com")),
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: Some(SorobanString::from_str(&env, "fr")),
    };
    
    let project_id = ProjectRegistry::register_project(&env, params).unwrap();
    
    // Update to clear language code
    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: Some(None),
    };
    
    let result = ProjectRegistry::update_project(&env, update_params);
    assert!(result.is_ok());
    
    let project = ProjectRegistry::get_project(&env, project_id).unwrap();
    assert_eq!(project.language_code, None);
}

#[test]
fn test_update_project_with_invalid_language_code() {
    let env = create_env();
    env.mock_all_auths();
    
    let owner = Address::generate(&env);
    
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: SorobanString::from_str(&env, "Test Project"),
        slug: SorobanString::from_str(&env, "test-project"),
        description: SorobanString::from_str(&env, "A test project"),
        category: SorobanString::from_str(&env, "DeFi"),
        website: Some(SorobanString::from_str(&env, "https://test.com")),
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
    
    let project_id = ProjectRegistry::register_project(&env, params).unwrap();
    
    // Attempt to update with invalid language code
    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: Some(Some(SorobanString::from_str(&env, "123"))), // Invalid: numbers
    };
    
    let result = ProjectRegistry::update_project(&env, update_params);
    assert_eq!(result, Err(ContractError::InvalidLanguageCode));
}

// ── Review Language Code Tests ──

// Note: Review tests would require more setup (initializing contract, fee config, etc.)
// For now, we've covered the critical validation logic. Full integration tests
// would go in the integration test suite.

#[test]
fn test_language_code_edge_cases() {
    let env = create_env();
    
    // All lowercase letters should work
    for byte1 in b'a'..=b'z' {
        for byte2 in b'a'..=b'z' {
            let code_str = format!("{}{}", byte1 as char, byte2 as char);
            let code = SorobanString::from_str(&env, &code_str);
            assert!(Utils::validate_language_code(&code).is_ok(), 
                "Code {} should be valid", code_str);
        }
    }
}

#[test]
fn test_language_code_boundary_values() {
    let env = create_env();
    
    // First and last valid lowercase letters
    let code = SorobanString::from_str(&env, "aa");
    assert!(Utils::validate_language_code(&code).is_ok());
    
    let code = SorobanString::from_str(&env, "zz");
    assert!(Utils::validate_language_code(&code).is_ok());
    
    // Just before and after lowercase range
    // 'a' = 97, 'z' = 122 in ASCII
    // '@' = 96, '{' = 123
    let bytes = soroban_sdk::Bytes::from_array(&env, &[96u8, 97u8]); // '@a'
    let code = SorobanString::from_bytes(&bytes);
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
    
    let bytes = soroban_sdk::Bytes::from_array(&env, &[97u8, 123u8]); // 'a{'
    let code = SorobanString::from_bytes(&bytes);
    assert_eq!(Utils::validate_language_code(&code), Err(ContractError::InvalidLanguageCode));
}
