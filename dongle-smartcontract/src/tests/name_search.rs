//! Tests for case-insensitive project lookup by name (get_project_by_name).

use soroban_sdk::{testutils::Address as _, Address, String};

use super::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectUpdateParams;

#[test]
fn test_get_project_by_name_exact_match() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let name = String::from_str(&env, "TestProject");
    let project = client.get_project_by_name(&name).unwrap();
    assert_eq!(project.id, project_id);
}

#[test]
fn test_get_project_by_name_case_insensitive() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    for variant in ["testproject", "TESTPROJECT", "TeStPrOjEcT"] {
        let name = String::from_str(&env, variant);
        let project = client.get_project_by_name(&name).unwrap();
        assert_eq!(project.id, project_id);
    }
}

#[test]
fn test_get_project_by_nonexistent_name() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let name = String::from_str(&env, "doesnotexist");
    assert!(client.get_project_by_name(&name).is_none());
}

#[test]
fn test_get_project_by_name_avoids_full_scan_semantics() {
    // Distinct names should resolve to their own project, not to whichever
    // project happens to be registered last.
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let id_a = create_test_project(&client, &owner, "AlphaProject");
    let id_b = create_test_project(&client, &owner, "BetaProject");
    let id_c = create_test_project(&client, &owner, "GammaProject");

    let a = client
        .get_project_by_name(&String::from_str(&env, "alphaproject"))
        .unwrap();
    let b = client
        .get_project_by_name(&String::from_str(&env, "BETAPROJECT"))
        .unwrap();
    let c = client
        .get_project_by_name(&String::from_str(&env, "GammaProject"))
        .unwrap();

    assert_eq!(a.id, id_a);
    assert_eq!(b.id, id_b);
    assert_eq!(c.id, id_c);
}

#[test]
fn test_get_project_by_name_after_rename() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "OldName");

    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "NewName")),
        slug: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
    };
    client.mock_all_auths().update_project(&update_params);

    // Old name (in any case) no longer resolves.
    assert!(client
        .get_project_by_name(&String::from_str(&env, "oldname"))
        .is_none());

    // New name resolves case-insensitively.
    let project = client
        .get_project_by_name(&String::from_str(&env, "newname"))
        .unwrap();
    assert_eq!(project.id, project_id);
}
