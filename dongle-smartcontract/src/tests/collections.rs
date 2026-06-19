use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

#[test]
fn test_create_collection_admin_only() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);

    let result = client.mock_all_auths().try_create_collection(
        &non_admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "DeFi projects"),
    );

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_update_collection_admin_only() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "DeFi projects"),
    );

    let result = client.mock_all_auths().try_update_collection(
        &non_admin,
        &id,
        &String::from_str(&env, "DeFi 2"),
        &String::from_str(&env, "Updated"),
    );

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_delete_collection_admin_only() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "DeFi projects"),
    );

    let result = client
        .mock_all_auths()
        .try_delete_collection(&non_admin, &id);

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_add_project_to_collection_admin_only() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");
    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );

    let result = client.mock_all_auths().try_add_project_to_collection(
        &non_admin,
        &collection_id,
        &project_id,
    );

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_remove_project_from_collection_admin_only() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");
    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &collection_id, &project_id);

    let result = client.mock_all_auths().try_remove_project_from_collection(
        &non_admin,
        &collection_id,
        &project_id,
    );

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_create_and_get_collection() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "Decentralized finance projects"),
    );

    assert_eq!(id, 1);

    let collection = client.get_collection(&id);
    assert_eq!(collection.id, 1);
    assert_eq!(collection.name, String::from_str(&env, "DeFi"));
    assert_eq!(
        collection.description,
        String::from_str(&env, "Decentralized finance projects")
    );
    assert!(collection.created_at > 0);
    assert_eq!(collection.created_at, collection.updated_at);
}

#[test]
fn test_get_collection_not_found() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let result = client.try_get_collection(&999u64);
    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

#[test]
fn test_create_duplicate_collection_name() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "DeFi projects"),
    );

    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "More DeFi"),
    );

    assert_eq!(result, Err(Ok(ContractError::CollectionAlreadyExists)));
}

#[test]
fn test_update_collection() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "DeFi projects"),
    );

    client.mock_all_auths().update_collection(
        &admin,
        &id,
        &String::from_str(&env, "DeFi 2.0"),
        &String::from_str(&env, "Updated description"),
    );

    let collection = client.get_collection(&id);
    assert_eq!(collection.name, String::from_str(&env, "DeFi 2.0"));
    assert_eq!(
        collection.description,
        String::from_str(&env, "Updated description")
    );
}

#[test]
fn test_update_collection_not_found() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client.mock_all_auths().try_update_collection(
        &admin,
        &999u64,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "Desc"),
    );

    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

#[test]
fn test_delete_collection() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "DeFi projects"),
    );

    client.mock_all_auths().delete_collection(&admin, &id);

    let result = client.try_get_collection(&id);
    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));

    let collections = client.list_collections(&0, &10);
    assert_eq!(collections.len(), 0);
}

#[test]
fn test_delete_collection_not_found() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client
        .mock_all_auths()
        .try_delete_collection(&admin, &999u64);

    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

#[test]
fn test_add_and_remove_project() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");
    let project2_id = create_test_project(&client, &owner, "Beta");

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top Projects"),
        &String::from_str(&env, "Best projects"),
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &collection_id, &project_id);
    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &collection_id, &project2_id);

    assert_eq!(client.get_collection_project_count(&collection_id), 2);

    let pids = client.list_collection_projects(&collection_id, &0, &10);
    assert_eq!(pids.len(), 2);
    assert_eq!(pids.get(0).unwrap(), project_id);
    assert_eq!(pids.get(1).unwrap(), project2_id);

    client
        .mock_all_auths()
        .remove_project_from_collection(&admin, &collection_id, &project_id);

    assert_eq!(client.get_collection_project_count(&collection_id), 1);

    let pids = client.list_collection_projects(&collection_id, &0, &10);
    assert_eq!(pids.len(), 1);
    assert_eq!(pids.get(0).unwrap(), project2_id);
}

#[test]
fn test_add_project_not_found() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top Projects"),
        &String::from_str(&env, "Best projects"),
    );

    let result =
        client
            .mock_all_auths()
            .try_add_project_to_collection(&admin, &collection_id, &999u64);

    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_add_project_to_nonexistent_collection() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");

    let result =
        client
            .mock_all_auths()
            .try_add_project_to_collection(&admin, &999u64, &project_id);

    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

#[test]
fn test_add_duplicate_project_to_collection() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top Projects"),
        &String::from_str(&env, "Best projects"),
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &collection_id, &project_id);

    let result =
        client
            .mock_all_auths()
            .try_add_project_to_collection(&admin, &collection_id, &project_id);

    assert_eq!(result, Err(Ok(ContractError::AlreadyInCollection)));
}

#[test]
fn test_remove_project_not_in_collection() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top Projects"),
        &String::from_str(&env, "Best projects"),
    );

    let result = client.mock_all_auths().try_remove_project_from_collection(
        &admin,
        &collection_id,
        &project_id,
    );

    assert_eq!(result, Err(Ok(ContractError::NotInCollection)));
}

#[test]
fn test_remove_project_from_nonexistent_collection() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");

    let result =
        client
            .mock_all_auths()
            .try_remove_project_from_collection(&admin, &999u64, &project_id);

    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

#[test]
fn test_list_collections_pagination() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let names = ["A", "B", "C", "D", "E"];
    for name in &names {
        client.mock_all_auths().create_collection(
            &admin,
            &String::from_str(&env, name),
            &String::from_str(&env, "desc"),
        );
    }

    let page1 = client.list_collections(&0, &3);
    let page2 = client.list_collections(&3, &3);

    assert_eq!(page1.len(), 3);
    assert_eq!(page1.get(0).unwrap().name, String::from_str(&env, "A"));
    assert_eq!(page1.get(1).unwrap().name, String::from_str(&env, "B"));
    assert_eq!(page1.get(2).unwrap().name, String::from_str(&env, "C"));

    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap().name, String::from_str(&env, "D"));
    assert_eq!(page2.get(1).unwrap().name, String::from_str(&env, "E"));
}

#[test]
fn test_list_collections_empty() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let collections = client.list_collections(&0, &10);
    assert_eq!(collections.len(), 0);
}

#[test]
fn test_list_collection_projects_pagination() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top Projects"),
        &String::from_str(&env, "Best projects"),
    );

    let mut project_ids: Vec<u64> = Vec::new(&env);
    for i in 0..5u64 {
        let name = match i {
            0 => "Alpha",
            1 => "Beta",
            2 => "Gamma",
            3 => "Delta",
            _ => "Epsilon",
        };
        let pid = create_test_project(&client, &owner, name);
        project_ids.push_back(pid);
        client
            .mock_all_auths()
            .add_project_to_collection(&admin, &collection_id, &pid);
    }

    let page1 = client.list_collection_projects(&collection_id, &0, &3);
    let page2 = client.list_collection_projects(&collection_id, &3, &3);

    assert_eq!(page1.len(), 3);
    assert_eq!(page1.get(0).unwrap(), project_ids.get(0).unwrap());
    assert_eq!(page1.get(1).unwrap(), project_ids.get(1).unwrap());
    assert_eq!(page1.get(2).unwrap(), project_ids.get(2).unwrap());

    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap(), project_ids.get(3).unwrap());
    assert_eq!(page2.get(1).unwrap(), project_ids.get(4).unwrap());
}

#[test]
fn test_list_collection_projects_empty() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Empty"),
        &String::from_str(&env, "Empty collection"),
    );

    let pids = client.list_collection_projects(&collection_id, &0, &10);
    assert_eq!(pids.len(), 0);
}

#[test]
fn test_collection_count() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    assert_eq!(client.get_collection_count(), 0);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "DeFi projects"),
    );
    assert_eq!(client.get_collection_count(), 1);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );
    assert_eq!(client.get_collection_count(), 2);
}

#[test]
fn test_collection_name_too_long() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let long_name = "a".repeat(101);
    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, &long_name),
        &String::from_str(&env, "desc"),
    );

    assert_eq!(result, Err(Ok(ContractError::ProjectNameTooLong)));
}

#[test]
fn test_collection_description_too_long() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let long_desc = "a".repeat(501);
    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, &long_desc),
    );

    assert_eq!(result, Err(Ok(ContractError::ProjectDescriptionTooLong)));
}

#[test]
fn test_empty_collection_name() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, ""),
        &String::from_str(&env, "desc"),
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData)));
}

#[test]
fn test_empty_collection_description() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, ""),
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData)));
}

#[test]
fn test_collection_project_count() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top Projects"),
        &String::from_str(&env, "desc"),
    );

    assert_eq!(client.get_collection_project_count(&collection_id), 0);

    let pid1 = create_test_project(&client, &owner, "Alpha");
    let pid2 = create_test_project(&client, &owner, "Beta");

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &collection_id, &pid1);
    assert_eq!(client.get_collection_project_count(&collection_id), 1);

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &collection_id, &pid2);
    assert_eq!(client.get_collection_project_count(&collection_id), 2);

    client
        .mock_all_auths()
        .remove_project_from_collection(&admin, &collection_id, &pid1);
    assert_eq!(client.get_collection_project_count(&collection_id), 1);
}

#[test]
fn test_update_collection_duplicate_name() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "DeFi projects"),
    );

    let id2 = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "Wallet projects"),
    );

    let result = client.mock_all_auths().try_update_collection(
        &admin,
        &id2,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "Updated"),
    );

    assert_eq!(result, Err(Ok(ContractError::CollectionAlreadyExists)));
}

#[test]
fn test_get_collection_count() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    assert_eq!(client.get_collection_count(), 0);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "A"),
        &String::from_str(&env, "a"),
    );
    assert_eq!(client.get_collection_count(), 1);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "B"),
        &String::from_str(&env, "b"),
    );
    assert_eq!(client.get_collection_count(), 2);
}

#[test]
fn test_delete_collection_removes_project_associations() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");

    let collection_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top Projects"),
        &String::from_str(&env, "Best"),
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &collection_id, &project_id);

    assert_eq!(client.get_collection_project_count(&collection_id), 1);

    client
        .mock_all_auths()
        .delete_collection(&admin, &collection_id);

    let result = client.try_get_collection(&collection_id);
    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));

    let project = client.get_project(&project_id);
    assert_eq!(project.unwrap().id, project_id);
}

#[test]
fn test_collection_max_limit_enforced() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    for i in 0..100u64 {
        extern crate alloc;
        use alloc::format;
        let name = format!("Collection-{}", i);
        client.mock_all_auths().create_collection(
            &admin,
            &String::from_str(&env, &name),
            &String::from_str(&env, "desc"),
        );
    }

    assert_eq!(client.get_collection_count(), 100);

    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, "Overflow"),
        &String::from_str(&env, "Too many"),
    );

    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded)));
}
