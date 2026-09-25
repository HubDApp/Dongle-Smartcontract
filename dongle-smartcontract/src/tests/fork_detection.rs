//! Focused tests for fork and migration detection (issue #748).

use crate::errors::ContractError;
use crate::fork_types::ForkSignal;
use crate::tests::fixtures::setup_contract;
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn register_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
    repository_url: Option<&str>,
) -> u64 {
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            slug: String::from_str(env, &name.to_lowercase()),
            description: String::from_str(env, "Fork detection test project"),
            category: String::from_str(env, "Infrastructure"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: repository_url.map(|url| String::from_str(env, url)),
        })
}

#[test]
fn detects_same_owner_name_and_repository_signals() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let parent = register_project(
        &client,
        &env,
        &owner,
        "alpha-core",
        Some("https://github.com/example/alpha"),
    );
    let child = register_project(
        &client,
        &env,
        &owner,
        "alpha-fork",
        Some("https://github.com/example/alpha"),
    );

    let detection = client.detect_project_fork(&child, &parent).unwrap();
    assert!(detection.signals.contains(&ForkSignal::SameOwner));
    assert!(detection.signals.contains(&ForkSignal::SimilarName));
    assert!(detection.signals.contains(&ForkSignal::SameRepository));
    assert_eq!(detection.confidence_bps, 10_000);
    assert!(detection.shared_team_members.contains(&owner));
}

#[test]
fn detects_shared_maintainer_across_different_teams() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let parent_owner = Address::generate(&env);
    let child_owner = Address::generate(&env);
    let maintainer = Address::generate(&env);
    let parent = register_project(&client, &env, &parent_owner, "ledger", None);
    let child = register_project(&client, &env, &child_owner, "wallet", None);

    client
        .mock_all_auths()
        .add_maintainer(&parent, &parent_owner, &maintainer);
    client
        .mock_all_auths()
        .add_maintainer(&child, &child_owner, &maintainer);

    let detection = client.detect_project_fork(&child, &parent).unwrap();
    assert!(detection.signals.contains(&ForkSignal::SharedMaintainer));
    assert!(detection.shared_team_members.contains(&maintainer));
}

#[test]
fn links_parent_and_child_with_review_snapshot() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let parent = register_project(
        &client,
        &env,
        &owner,
        "source",
        Some("https://github.com/example/source"),
    );
    let child = register_project(
        &client,
        &env,
        &owner,
        "destination",
        Some("https://github.com/example/source"),
    );
    let reviewer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_review(&parent, &reviewer, &5u32, &None);

    let relationship = client
        .mock_all_auths()
        .link_project_fork(&child, &parent, &owner, &true)
        .unwrap();

    assert_eq!(relationship.child_project_id, child);
    assert_eq!(relationship.parent_project_id, parent);
    let merged = relationship.merged_data.unwrap();
    assert_eq!(merged.review_stats.review_count, 1);
    assert_eq!(merged.review_stats.rating_sum, 5);
    assert_eq!(client.get_fork_parent(&child), Some(parent));

    let children = client.list_fork_children(&parent, &0, &10);
    assert_eq!(children.len(), 1);
    assert_eq!(children.get(0), Some(child));
    assert_eq!(
        client
            .get_fork_relationship(&child)
            .unwrap()
            .parent_project_id,
        parent
    );
}

#[test]
fn unrelated_projects_cannot_be_linked() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let parent_owner = Address::generate(&env);
    let child_owner = Address::generate(&env);
    let parent = register_project(&client, &env, &parent_owner, "ledger", None);
    let child = register_project(&client, &env, &child_owner, "wallet", None);

    let detection = client.detect_project_fork(&child, &parent).unwrap();
    assert!(detection.signals.is_empty());
    assert_eq!(
        client
            .mock_all_auths()
            .try_link_project_fork(&child, &parent, &child_owner, &false),
        Err(Ok(ContractError::InvalidInput))
    );
}

#[test]
fn rejects_unauthorized_duplicate_and_cyclic_links() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let first = register_project(
        &client,
        &env,
        &owner,
        "source",
        Some("https://github.com/example/project"),
    );
    let second = register_project(
        &client,
        &env,
        &owner,
        "destination",
        Some("https://github.com/example/project"),
    );

    assert_eq!(
        client
            .mock_all_auths()
            .try_link_project_fork(&second, &first, &stranger, &false),
        Err(Ok(ContractError::Unauthorized))
    );

    client
        .mock_all_auths()
        .link_project_fork(&second, &first, &owner, &false);
    assert_eq!(
        client
            .mock_all_auths()
            .try_link_project_fork(&second, &first, &owner, &false),
        Err(Ok(ContractError::AlreadyLinked))
    );
    assert_eq!(
        client
            .mock_all_auths()
            .try_link_project_fork(&first, &second, &owner, &false),
        Err(Ok(ContractError::CircularDependency))
    );
}

#[test]
fn self_detection_is_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project = register_project(&client, &env, &owner, "source", None);

    assert_eq!(
        client.try_detect_project_fork(&project, &project),
        Err(Ok(ContractError::CannotLinkToSelf))
    );
}
