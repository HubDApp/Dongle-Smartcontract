//! Dynamic category CRUD, hierarchy, stats, and migration tests (issue #752).

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn register_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
    category: &str,
) -> u64 {
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            slug: String::from_str(env, &name.to_lowercase()),
            description: String::from_str(env, "Dynamic category test project"),
            category: String::from_str(env, category),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
        })
}

#[test]
fn admin_manages_hierarchy_rename_and_stats() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);

    assert_eq!(
        client.mock_all_auths().try_create_project_category(
            &stranger,
            &String::from_str(&env, "Finance"),
            &String::from_str(&env, "root"),
            &None,
        ),
        Err(Ok(ContractError::Unauthorized))
    );

    let root = client
        .mock_all_auths()
        .create_project_category(
            &admin,
            &String::from_str(&env, "Finance"),
            &String::from_str(&env, "Root category"),
            &None,
        )
        .unwrap();
    let child = client
        .mock_all_auths()
        .create_project_category(
            &admin,
            &String::from_str(&env, "Payments"),
            &String::from_str(&env, "Payment protocols"),
            &Some(root.id),
        )
        .unwrap();

    let root_project = register_project(&client, &env, &owner, "RootProject", "Finance");
    let child_project = register_project(&client, &env, &owner, "ChildProject", "Payments");
    let root_stats = client.get_project_category_stats(root.id).unwrap();
    assert_eq!(root_stats.direct_project_count, 1);
    assert_eq!(root_stats.direct_active_project_count, 1);
    assert_eq!(root_stats.direct_child_count, 1);
    assert_eq!(root_stats.descendant_project_count, 1);
    assert_eq!(root_stats.hierarchy_active_project_count, 2);

    client
        .mock_all_auths()
        .archive_project(&child_project, &owner);
    assert_eq!(
        client
            .get_project_category_stats(root.id)
            .unwrap()
            .hierarchy_active_project_count,
        1
    );

    let renamed = client
        .mock_all_auths()
        .update_project_category(
            &admin,
            &child.id,
            &String::from_str(&env, "PaymentRails"),
            &String::from_str(&env, "Updated description"),
            &Some(root.id),
            &true,
        )
        .unwrap();
    assert_eq!(renamed.name, String::from_str(&env, "PaymentRails"));
    assert_eq!(
        client.get_project(child_project).unwrap().category,
        String::from_str(&env, "PaymentRails")
    );
    assert_eq!(
        client
            .get_project_category_stats(root.id)
            .unwrap()
            .descendant_project_count,
        1
    );
    assert_eq!(client.list_project_category_children(root.id).len(), 1);
    assert_eq!(client.list_project_categories(&0, &10).len(), 2);
    assert!(root_project < child_project);

    assert_eq!(
        client.mock_all_auths().try_update_project_category(
            &admin,
            &child.id,
            &String::from_str(&env, "PaymentRails"),
            &String::from_str(&env, "cannot deactivate"),
            &Some(root.id),
            &false,
        ),
        Err(Ok(ContractError::ProjectCategoryHasProjects))
    );
}

#[test]
fn deletion_migrates_projects_and_reparents_children() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let source = client
        .mock_all_auths()
        .create_project_category(
            &admin,
            &String::from_str(&env, "Source"),
            &String::from_str(&env, "Source"),
            &None,
        )
        .unwrap();
    let target = client
        .mock_all_auths()
        .create_project_category(
            &admin,
            &String::from_str(&env, "Target"),
            &String::from_str(&env, "Target"),
            &None,
        )
        .unwrap();
    let grandchild = client
        .mock_all_auths()
        .create_project_category(
            &admin,
            &String::from_str(&env, "Grandchild"),
            &String::from_str(&env, "Grandchild"),
            &Some(source.id),
        )
        .unwrap();
    let first = register_project(&client, &env, &owner, "MigrateOne", "Source");
    let second = register_project(&client, &env, &owner, "MigrateTwo", "Source");

    assert_eq!(
        client
            .mock_all_auths()
            .try_delete_project_category(&admin, &source.id, &None),
        Err(Ok(ContractError::ProjectCategoryHasProjects))
    );
    let result = client
        .mock_all_auths()
        .delete_project_category(&admin, &source.id, &Some(target.id))
        .unwrap();
    assert_eq!(result.migrated_project_count, 2);
    assert_eq!(result.reparented_child_count, 1);
    assert_eq!(result.migration_target_id, Some(target.id));
    assert!(client.get_project_category(source.id).is_none());
    assert_eq!(
        client.get_project(first).unwrap().category,
        String::from_str(&env, "Target")
    );
    assert_eq!(
        client.get_project(second).unwrap().category,
        String::from_str(&env, "Target")
    );
    assert_eq!(
        client
            .get_project_category(grandchild.id)
            .unwrap()
            .parent_id,
        Some(target.id)
    );

    let target_stats = client.get_project_category_stats(target.id).unwrap();
    assert_eq!(target_stats.direct_project_count, 2);
    assert_eq!(target_stats.direct_child_count, 1);

    assert_eq!(
        client.mock_all_auths().try_update_project_category(
            &admin,
            &target.id,
            &String::from_str(&env, "Target"),
            &String::from_str(&env, "Target"),
            &Some(grandchild.id),
            &true,
        ),
        Err(Ok(ContractError::ProjectCategoryHierarchyInvalid))
    );
}

#[test]
fn inactive_categories_reject_projects_but_legacy_labels_remain_valid() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let category = client
        .mock_all_auths()
        .create_project_category(
            &admin,
            &String::from_str(&env, "TemporarilyClosed"),
            &String::from_str(&env, "Can be disabled"),
            &None,
        )
        .unwrap();
    client
        .mock_all_auths()
        .update_project_category(
            &admin,
            &category.id,
            &String::from_str(&env, "TemporarilyClosed"),
            &String::from_str(&env, "Can be disabled"),
            &None,
            &false,
        )
        .unwrap();
    assert_eq!(
        client.mock_all_auths().try_create_project_category(
            &admin,
            &String::from_str(&env, "temporarilyclosed"),
            &String::from_str(&env, "duplicate"),
            &None,
        ),
        Err(Ok(ContractError::ProjectCategoryAlreadyExists))
    );
    assert_eq!(
        client
            .mock_all_auths()
            .try_register_project(&ProjectRegistrationParams {
                owner: owner.clone(),
                name: String::from_str(&env, "BlockedProject"),
                slug: String::from_str(&env, "blocked-project"),
                description: String::from_str(&env, "Should be blocked"),
                category: String::from_str(&env, "TemporarilyClosed"),
                website: None,
                license: None,
                logo_cid: None,
                metadata_cid: None,
                tags: None,
                social_links: None,
                launch_timestamp: None,
                bounty_url: None,
                repository_url: None,
            }),
        Err(Ok(ContractError::ProjectCategoryInactive))
    );

    // Categories predating dynamic management remain accepted for compatibility,
    // and adopting one later backfills its existing project count.
    let legacy = register_project(&client, &env, &owner, "LegacyProject", "LegacyCategory");
    assert!(legacy > 0);
    let managed_legacy = client
        .mock_all_auths()
        .create_project_category(
            &admin,
            &String::from_str(&env, "LegacyCategory"),
            &String::from_str(&env, "Adopted legacy category"),
            &None,
        )
        .unwrap();
    assert_eq!(
        client
            .get_project_category_stats(managed_legacy.id)
            .unwrap()
            .direct_project_count,
        1
    );
}
