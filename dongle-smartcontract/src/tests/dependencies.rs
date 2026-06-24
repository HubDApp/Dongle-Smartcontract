#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{DependencyRef, ProjectDependency, ProjectRegistrationParams};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn mk_external_cid(env: &Env) -> String {
    String::from_str(env, "QmExternalCid123456789012345678901234567890123456")
}

fn mk_external_url(env: &Env) -> String {
    String::from_str(env, "https://example.com/dependency")
}

fn mk_dep(env: &Env, dep: DependencyRef, label: &str) -> ProjectDependency {
    ProjectDependency {
        reference: dep,
        label: Some(String::from_str(env, label)),
        metadata_cid: None,
        added_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
    }
}

#[test]
fn test_add_get_remove_project_dependencies_external_cid() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, owner) = setup_contract(&env);
    let project_id = create_test_project(&client, &owner, "DepProject1");

    let dep_ref = DependencyRef {
        project_id: None,
        external_cid: Some(mk_external_cid(&env)),
        external_url: None,
    };

    let dep = mk_dep(&env, dep_ref.clone(), "oracle");
    client.add_project_dependency(&project_id, &owner, &dep);

    let deps = client.get_project_dependencies(&project_id);
    assert_eq!(deps.len(), 1);
    assert_eq!(
        deps.get(0).unwrap().label.as_ref().unwrap(),
        &String::from_str(&env, "oracle")
    );

    client.remove_project_dependency(&project_id, &owner, &dep_ref);
    let deps_after = client.get_project_dependencies(&project_id);
    assert_eq!(deps_after.len(), 0);
}

#[test]
fn test_add_dependency_internal_project_id() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, owner) = setup_contract(&env);

    let dep_project_id = create_test_project(&client, &owner, "DepTarget");
    let project_id = create_test_project(&client, &owner, "DepProject2");

    let dep_ref = DependencyRef {
        project_id: Some(dep_project_id),
        external_cid: None,
        external_url: None,
    };

    let dep = mk_dep(&env, dep_ref.clone(), "depends");
    client.add_project_dependency(&project_id, &owner, &dep);

    let deps = client.get_project_dependencies(&project_id);
    assert_eq!(deps.len(), 1);
    assert_eq!(
        deps.get(0).unwrap().reference.project_id,
        Some(dep_project_id)
    );
}

#[test]
fn test_reject_duplicate_dependency_external_url() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, owner) = setup_contract(&env);
    let project_id = create_test_project(&client, &owner, "DepProjectDup");

    let dep_ref = DependencyRef {
        project_id: None,
        external_cid: None,
        external_url: Some(mk_external_url(&env)),
    };

    let dep1 = mk_dep(&env, dep_ref.clone(), "a");
    client.add_project_dependency(&project_id, &owner, &dep1);

    let dep2 = mk_dep(&env, dep_ref.clone(), "b");
    let result = client.try_add_project_dependency(&project_id, &owner, &dep2);
    assert!(result.is_err());
}

#[test]
fn test_update_dependency_changes_metadata() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, owner) = setup_contract(&env);
    let project_id = create_test_project(&client, &owner, "DepProjectUpd");

    let dep_ref = DependencyRef {
        project_id: None,
        external_cid: Some(mk_external_cid(&env)),
        external_url: None,
    };

    let mut dep = mk_dep(&env, dep_ref.clone(), "old");
    dep.metadata_cid = Some(String::from_str(
        &env,
        "QmMetaOld123456789012345678901234567890123456",
    ));

    client.add_project_dependency(&project_id, &owner, &dep);

    let mut updated = dep.clone();
    updated.label = Some(String::from_str(&env, "new"));
    updated.metadata_cid = Some(String::from_str(
        &env,
        "QmMetaNew123456789012345678901234567890123456",
    ));
    updated.updated_at = env.ledger().timestamp();

    client.update_project_dependency(&project_id, &owner, &dep_ref, &updated);

    let deps = client.get_project_dependencies(&project_id);
    assert_eq!(deps.len(), 1);
    let stored = deps.get(0).unwrap();
    assert_eq!(stored.label.unwrap(), String::from_str(&env, "new"));
    assert_eq!(
        stored.metadata_cid.unwrap(),
        String::from_str(&env, "QmMetaNew123456789012345678901234567890123456")
    );
}

#[test]
fn test_invalid_dependency_reference_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, owner) = setup_contract(&env);
    let project_id = create_test_project(&client, &owner, "DepProjectInvalid");

    // missing all forms
    let bad_ref = DependencyRef {
        project_id: None,
        external_cid: None,
        external_url: None,
    };

    let bad_dep = ProjectDependency {
        reference: bad_ref,
        label: None,
        metadata_cid: None,
        added_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
    };

    let result = client.try_add_project_dependency(&project_id, &owner, &bad_dep);
    assert!(result.is_err());
}
