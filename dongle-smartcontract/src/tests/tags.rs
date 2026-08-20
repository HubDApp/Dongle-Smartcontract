//! Tag validation tests (#526).
//!
//! Covers count/length/charset rules and case-insensitive duplicate rejection.

extern crate alloc;
use alloc::string::String as StdString;

use crate::constants::{MAX_TAG_LENGTH, MAX_TAGS_PER_PROJECT};
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use crate::utils::Utils;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn mk_env() -> Env {
    Env::default()
}

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

fn tags(env: &Env, values: &[&str]) -> Vec<String> {
    let mut list = Vec::new(env);
    for value in values {
        list.push_back(s(env, value));
    }
    list
}

fn repeat_byte(env: &Env, ch: u8, n: usize) -> String {
    let raw: StdString = core::iter::repeat(ch as char).take(n).collect();
    String::from_str(env, &raw)
}

fn unique_tags(env: &Env, count: u32) -> Vec<String> {
    let mut list = Vec::new(env);
    for i in 0..count {
        let label = alloc::format!("tag{i}");
        list.push_back(s(env, &label));
    }
    list
}

fn registration_params(env: &Env, owner: &Address, name: &str, tag_list: Vec<String>) -> ProjectRegistrationParams {
    let slug = name.to_lowercase().replace(' ', "-");
    ProjectRegistrationParams {
        owner: owner.clone(),
        name: s(env, name),
        slug: s(env, &slug),
        description: s(env, "Tag validation test project."),
        category: s(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(tag_list),
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    }
}

fn update_params(env: &Env, project_id: u64, caller: &Address, tag_list: Vec<String>) -> ProjectUpdateParams {
    ProjectUpdateParams {
        project_id,
        caller: caller.clone(),
        name: None,
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(Some(tag_list)),
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Utils::validate_tags
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn unique_tags_are_valid() {
    let e = mk_env();
    assert!(Utils::validate_tags(&tags(&e, &["defi", "nft", "gaming"])).is_ok());
}

#[test]
fn empty_tag_list_is_valid() {
    let e = mk_env();
    assert!(Utils::validate_tags(&Vec::new(&e)).is_ok());
}

#[test]
fn exact_duplicate_tags_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_tags(&tags(&e, &["defi", "defi", "nft"])),
        Err(ContractError::InvalidTags)
    );
}

#[test]
fn case_insensitive_duplicate_tags_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_tags(&tags(&e, &["defi", "DeFi"])),
        Err(ContractError::InvalidTags)
    );
    assert_eq!(
        Utils::validate_tags(&tags(&e, &["NFT", "nft"])),
        Err(ContractError::InvalidTags)
    );
    assert_eq!(
        Utils::validate_tags(&tags(&e, &["Gaming", "GAMING", "nft"])),
        Err(ContractError::InvalidTags)
    );
}

#[test]
fn mixed_case_unique_tags_are_valid() {
    let e = mk_env();
    assert!(Utils::validate_tags(&tags(&e, &["DeFi", "NFT", "dao"])).is_ok());
}

#[test]
fn hyphen_and_underscore_are_not_duplicates() {
    let e = mk_env();
    assert!(Utils::validate_tags(&tags(&e, &["de-fi", "de_fi"])).is_ok());
}

#[test]
fn empty_tag_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_tags(&tags(&e, &[""])),
        Err(ContractError::InvalidTags)
    );
}

#[test]
fn invalid_charset_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_tags(&tags(&e, &["de fi"])),
        Err(ContractError::InvalidTags)
    );
    assert_eq!(
        Utils::validate_tags(&tags(&e, &["defi!"])),
        Err(ContractError::InvalidTags)
    );
}

#[test]
fn tag_at_max_length_valid() {
    let e = mk_env();
    let mut list = Vec::new(&e);
    list.push_back(repeat_byte(&e, b'a', MAX_TAG_LENGTH));
    assert!(Utils::validate_tags(&list).is_ok());
}

#[test]
fn tag_over_max_length_rejected() {
    let e = mk_env();
    let mut list = Vec::new(&e);
    list.push_back(repeat_byte(&e, b'a', MAX_TAG_LENGTH + 1));
    assert_eq!(
        Utils::validate_tags(&list),
        Err(ContractError::InvalidTags)
    );
}

#[test]
fn tag_count_at_max_valid() {
    let e = mk_env();
    assert!(Utils::validate_tags(&unique_tags(&e, MAX_TAGS_PER_PROJECT)).is_ok());
}

#[test]
fn tag_count_over_max_rejected() {
    let e = mk_env();
    assert_eq!(
        Utils::validate_tags(&unique_tags(&e, MAX_TAGS_PER_PROJECT + 1)),
        Err(ContractError::InvalidTags)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// register_project / update_project
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn register_project_rejects_duplicate_tags() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params = registration_params(&env, &owner, "DupTagsProject", tags(&env, &["defi", "defi", "nft"]));
    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::InvalidTags.into())));
}

#[test]
fn register_project_rejects_case_insensitive_duplicate_tags() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params = registration_params(&env, &owner, "CaseDupTags", tags(&env, &["defi", "DeFi"]));
    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::InvalidTags.into())));
}

#[test]
fn register_project_accepts_unique_tags() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params = registration_params(&env, &owner, "UniqueTags", tags(&env, &["defi", "nft"]));
    assert!(client.try_register_project(&params).is_ok());
}

#[test]
fn update_project_rejects_duplicate_tags() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "UpdateDupTags");

    let params = update_params(&env, project_id, &owner, tags(&env, &["nft", "NFT"]));
    let result = client.try_update_project(&params);
    assert_eq!(result, Err(Ok(ContractError::InvalidTags.into())));
}

#[test]
fn update_project_accepts_unique_tags() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "UpdateUniqueTags");

    let params = update_params(&env, project_id, &owner, tags(&env, &["defi", "nft"]));
    assert!(client.try_update_project(&params).is_ok());
}
