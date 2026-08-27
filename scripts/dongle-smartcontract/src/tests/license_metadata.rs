#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn register_project_with_valid_license_returns_it_in_reads() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let license = String::from_str(&env, "Apache-2.0");

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "LicensedProject"),
        slug: String::from_str(&env, "licensed-project"),
        description: String::from_str(&env, "Project with SPDX license metadata"),
        category: String::from_str(&env, "Infra"),
        website: None,
        license: Some(license.clone()),
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };

    let project_id = client.register_project(&params);
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.license, Some(license.clone()));

    let by_slug = client
        .get_project_by_slug(&String::from_str(&env, "licensed-project"))
        .unwrap();
    assert_eq!(by_slug.license, Some(license));
}

#[test]
fn register_project_without_license_keeps_reads_empty() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params = ProjectRegistrationParams {
        owner,
        name: String::from_str(&env, "UnlicensedProject"),
        slug: String::from_str(&env, "unlicensed-project"),
        description: String::from_str(&env, "Project without license metadata"),
        category: String::from_str(&env, "Infra"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };

    let project_id = client.register_project(&params);
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.license, None);
}

#[test]
fn update_project_rejects_invalid_license() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = client.register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "LicenseUpdateProject"),
        slug: String::from_str(&env, "license-update-project"),
        description: String::from_str(&env, "Project for invalid license update test"),
        category: String::from_str(&env, "Infra"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    });

    let result = client.try_update_project(&ProjectUpdateParams {
        project_id,
        caller: owner,
        name: None,
        slug: None,
        description: None,
        category: None,
        website: None,
        license: Some(Some(String::from_str(&env, "MIT OR Apache-2.0"))),
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    });

    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData.into())));
}
