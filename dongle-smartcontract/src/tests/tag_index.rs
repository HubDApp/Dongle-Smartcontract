//! Inverted tag index tests (#485).
//!
//! `list_projects_by_tag` reads the `TagProjects` index instead of walking the
//! whole project ID space, so these cover the index staying in step with
//! registration, tag updates, tag removal, and archival.

use crate::tests::fixtures::setup_contract;
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

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

fn registration_params(
    env: &Env,
    owner: &Address,
    name: &str,
    tag_list: Vec<String>,
) -> ProjectRegistrationParams {
    let slug = name.to_lowercase().replace(' ', "-");
    ProjectRegistrationParams {
        owner: owner.clone(),
        name: s(env, name),
        slug: s(env, &slug),
        description: s(env, "Tag index test project."),
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

fn retag_params(
    env: &Env,
    project_id: u64,
    caller: &Address,
    tag_list: Option<Vec<String>>,
) -> ProjectUpdateParams {
    let _ = env;
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
        tags: Some(tag_list),
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    }
}

#[test]
fn registered_projects_are_indexed_by_every_tag() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let a = client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Alpha",
            tags(&env, &["defi", "lending"]),
        ));
    let b = client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Beta",
            tags(&env, &["defi", "gaming"]),
        ));

    let defi = client.list_projects_by_tag(&s(&env, "defi"), &0, &10);
    assert_eq!(defi.len(), 2);
    assert_eq!(defi.get(0).unwrap().id, a);
    assert_eq!(defi.get(1).unwrap().id, b);

    let lending = client.list_projects_by_tag(&s(&env, "lending"), &0, &10);
    assert_eq!(lending.len(), 1);
    assert_eq!(lending.get(0).unwrap().id, a);
}

#[test]
fn unknown_tag_returns_nothing() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Alpha",
            tags(&env, &["defi"]),
        ));

    let found = client.list_projects_by_tag(&s(&env, "nonexistent"), &0, &10);
    assert_eq!(found.len(), 0);
}

#[test]
fn updating_tags_moves_the_project_between_index_entries() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Alpha",
            tags(&env, &["defi"]),
        ));

    client.mock_all_auths().update_project(&retag_params(
        &env,
        id,
        &owner,
        Some(tags(&env, &["gaming"])),
    ));

    assert_eq!(
        client.list_projects_by_tag(&s(&env, "defi"), &0, &10).len(),
        0
    );
    let gaming = client.list_projects_by_tag(&s(&env, "gaming"), &0, &10);
    assert_eq!(gaming.len(), 1);
    assert_eq!(gaming.get(0).unwrap().id, id);
}

// NOTE: clearing every tag (`tags: Some(None)`) is not covered here. Soroban
// collapses the nested `Option<Option<_>>` when `ProjectUpdateParams` crosses the
// contract boundary, so the client cannot express "set tags to None" at all — the
// update arrives as "tags untouched". `tag_index_sync` handles that case, but it is
// unreachable through the generated client, and the collapse predates this change.
#[test]
fn keeping_a_tag_while_adding_another_leaves_both_indexed() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Alpha",
            tags(&env, &["defi"]),
        ));

    client.mock_all_auths().update_project(&retag_params(
        &env,
        id,
        &owner,
        Some(tags(&env, &["defi", "nft"])),
    ));

    assert_eq!(
        client.list_projects_by_tag(&s(&env, "defi"), &0, &10).len(),
        1
    );
    assert_eq!(
        client.list_projects_by_tag(&s(&env, "nft"), &0, &10).len(),
        1
    );
}

#[test]
fn archived_projects_are_hidden_from_tag_listings() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Alpha",
            tags(&env, &["defi"]),
        ));

    client.mock_all_auths().archive_project(&id, &owner);

    assert_eq!(
        client.list_projects_by_tag(&s(&env, "defi"), &0, &10).len(),
        0
    );
}

#[test]
fn start_index_pages_through_matching_projects() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let a = client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Alpha",
            tags(&env, &["defi"]),
        ));
    let b = client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Beta",
            tags(&env, &["defi"]),
        ));
    let c = client
        .mock_all_auths()
        .register_project(&registration_params(
            &env,
            &owner,
            "Gamma",
            tags(&env, &["defi"]),
        ));

    let first = client.list_projects_by_tag(&s(&env, "defi"), &0, &2);
    assert_eq!(first.len(), 2);
    assert_eq!(first.get(0).unwrap().id, a);
    assert_eq!(first.get(1).unwrap().id, b);

    let second = client.list_projects_by_tag(&s(&env, "defi"), &2, &2);
    assert_eq!(second.len(), 1);
    assert_eq!(second.get(0).unwrap().id, c);
}
