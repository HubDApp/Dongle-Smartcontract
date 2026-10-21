//! Collection hierarchies and nesting (issue #905).
//!
//! Covers:
//! * `parent_collection_id` on the `Collection` record,
//! * the maximum nesting depth (`MAX_COLLECTION_DEPTH` = 5),
//! * the acyclic-graph invariant (no collection can become its own ancestor),
//! * `get_collection_tree` (nested structure of a subtree),
//! * `get_collection_path` (breadcrumb navigation).

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    setup_contract(env)
}

/// Create a top-level collection owned by `admin`.
fn create_root(
    client: &DongleContractClient<'_>,
    env: &Env,
    admin: &Address,
    name: &str,
) -> u64 {
    client.mock_all_auths().create_collection(
        admin,
        &String::from_str(env, name),
        &String::from_str(env, "root collection"),
    )
}

/// Create a collection nested inside `parent_id`.
fn create_child(
    client: &DongleContractClient<'_>,
    env: &Env,
    admin: &Address,
    name: &str,
    parent_id: u64,
) -> u64 {
    client.mock_all_auths().create_nested_collection(
        admin,
        &String::from_str(env, name),
        &String::from_str(env, "nested collection"),
        &true,
        &parent_id,
    )
}

#[test]
fn test_top_level_collection_has_no_parent() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let root = create_root(&client, &env, &admin, "Root");

    let collection = client.get_collection(&root).unwrap();
    assert_eq!(collection.parent_collection_id, None);
    assert_eq!(client.get_collection_depth(&root), 1);
}

#[test]
fn test_nested_collection_records_its_parent() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let root = create_root(&client, &env, &admin, "Root");
    let child = create_child(&client, &env, &admin, "Child", root);

    let collection = client.get_collection(&child).unwrap();
    assert_eq!(collection.parent_collection_id, Some(root));
    assert_eq!(client.get_collection_depth(&child), 2);

    let children = client.get_collection_children(&root);
    assert_eq!(children.len(), 1);
    assert_eq!(children.get(0), Some(child));
}

#[test]
fn test_depth_limit_is_enforced() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let level1 = create_root(&client, &env, &admin, "Level1");
    let level2 = create_child(&client, &env, &admin, "Level2", level1);
    let level3 = create_child(&client, &env, &admin, "Level3", level2);
    let level4 = create_child(&client, &env, &admin, "Level4", level3);
    let level5 = create_child(&client, &env, &admin, "Level5", level4);

    assert_eq!(client.get_collection_depth(&level5), 5);

    // A sixth level is rejected.
    let result = client.mock_all_auths().try_create_nested_collection(
        &admin,
        &String::from_str(&env, "Level6"),
        &String::from_str(&env, "too deep"),
        &true,
        &level5,
    );
    assert_eq!(result, Err(Ok(ContractError::CollectionDepthExceeded)));
}

#[test]
fn test_a_collection_cannot_become_its_own_ancestor() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let root = create_root(&client, &env, &admin, "Root");
    let child = create_child(&client, &env, &admin, "Child", root);

    // Self-parenting is a cycle.
    let result = client
        .mock_all_auths()
        .try_set_collection_parent(&admin, &child, &Some(child));
    assert_eq!(result, Err(Ok(ContractError::CollectionHierarchyCycle)));

    // Re-parenting the root under its own child is a cycle too.
    let result = client
        .mock_all_auths()
        .try_set_collection_parent(&admin, &root, &Some(child));
    assert_eq!(result, Err(Ok(ContractError::CollectionHierarchyCycle)));
}

#[test]
fn test_collection_can_be_moved_between_parents() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let root_a = create_root(&client, &env, &admin, "RootA");
    let root_b = create_root(&client, &env, &admin, "RootB");
    let child = create_child(&client, &env, &admin, "Child", root_a);

    client
        .mock_all_auths()
        .set_collection_parent(&admin, &child, &Some(root_b));

    let collection = client.get_collection(&child).unwrap();
    assert_eq!(collection.parent_collection_id, Some(root_b));

    let new_children = client.get_collection_children(&root_b);
    assert_eq!(new_children.len(), 1);
    assert_eq!(new_children.get(0), Some(child));
    assert!(client.get_collection_children(&root_a).is_empty());
}

#[test]
fn test_get_collection_tree_returns_the_nested_structure() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let root = create_root(&client, &env, &admin, "Root");
    let child_a = create_child(&client, &env, &admin, "ChildA", root);
    let child_b = create_child(&client, &env, &admin, "ChildB", root);
    let grandchild = create_child(&client, &env, &admin, "GrandChild", child_a);

    let tree = client.get_collection_tree(&root);
    assert_eq!(tree.len(), 4);

    // Root first.
    assert_eq!(tree.get(0).unwrap().collection.id, root);
    assert_eq!(tree.get(0).unwrap().depth, 0);
    assert_eq!(tree.get(0).unwrap().parent_id, None);
    let root_children = tree.get(0).unwrap().child_ids.clone();
    assert_eq!(root_children.len(), 2);
    assert_eq!(root_children.get(0), Some(child_a));
    assert_eq!(root_children.get(1), Some(child_b));

    // Children of the root come next.
    assert_eq!(tree.get(1).unwrap().collection.id, child_a);
    assert_eq!(tree.get(1).unwrap().depth, 1);
    assert_eq!(tree.get(1).unwrap().parent_id, Some(root));
    let child_a_children = tree.get(1).unwrap().child_ids.clone();
    assert_eq!(child_a_children.len(), 1);
    assert_eq!(child_a_children.get(0), Some(grandchild));

    // The grandchild is the deepest node.
    assert_eq!(tree.get(3).unwrap().collection.id, grandchild);
    assert_eq!(tree.get(3).unwrap().depth, 2);
    assert_eq!(tree.get(3).unwrap().parent_id, Some(child_a));
}

#[test]
fn test_get_collection_path_returns_the_breadcrumb() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let root = create_root(&client, &env, &admin, "Root");
    let child = create_child(&client, &env, &admin, "Child", root);
    let grandchild = create_child(&client, &env, &admin, "GrandChild", child);

    let path = client.get_collection_path(&grandchild);
    assert_eq!(path.len(), 3);
    assert_eq!(path.get(0).unwrap().id, root);
    assert_eq!(path.get(1).unwrap().id, child);
    assert_eq!(path.get(2).unwrap().id, grandchild);

    // The root has a single-element path.
    let root_path = client.get_collection_path(&root);
    assert_eq!(root_path.len(), 1);
    assert_eq!(root_path.get(0).unwrap().id, root);
}

#[test]
fn test_hierarchy_is_acyclic() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let root = create_root(&client, &env, &admin, "Root");
    let child = create_child(&client, &env, &admin, "Child", root);
    let grandchild = create_child(&client, &env, &admin, "GrandChild", child);

    // Walk up from every collection: the chain must terminate and must never
    // contain the same collection twice.
    let all: std::vec::Vec<u64> = client
        .list_collections(&0u32, &100u32)
        .iter()
        .map(|c| c.id)
        .collect();

    for id in all.iter() {
        let path = client.get_collection_path(id);
        assert!(path.len() <= 5, "path longer than the maximum depth");

        let mut seen: Vec<u64> = Vec::new(&env);
        for node in path.iter() {
            assert!(
                !seen.iter().any(|x| *x == node.id),
                "cycle detected in the collection hierarchy"
            );
            seen.push_back(node.id);
        }
    }

    // The deepest collection is still reachable from the root.
    let tree = client.get_collection_tree(&root);
    assert!(tree.iter().any(|node| node.collection.id == grandchild));
}

#[test]
fn test_collection_with_children_cannot_be_deleted() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let root = create_root(&client, &env, &admin, "Root");
    let child = create_child(&client, &env, &admin, "Child", root);

    let result = client.mock_all_auths().try_delete_collection(&admin, &root);
    assert_eq!(result, Err(Ok(ContractError::CollectionHasChildren)));

    // The leaf can be deleted; the parent then has no children left.
    client.mock_all_auths().delete_collection(&admin, &child);
    assert!(client.get_collection_children(&root).is_empty());
    client.mock_all_auths().delete_collection(&admin, &root);
    assert!(client.get_collection(&root).is_none());
}

#[test]
fn test_nested_collection_requires_an_existing_parent() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let result = client.mock_all_auths().try_create_nested_collection(
        &admin,
        &String::from_str(&env, "Orphan"),
        &String::from_str(&env, "no parent"),
        &true,
        &999u64,
    );

    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}
