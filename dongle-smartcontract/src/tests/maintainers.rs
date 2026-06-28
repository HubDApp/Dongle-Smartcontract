#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectUpdateParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_owner_can_add_and_remove_maintainers() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let maintainer = Address::generate(&env);

    // Verify initially empty
    let list = client.get_maintainers(&project_id);
    assert_eq!(list.len(), 0);

    // Owner adds maintainer
    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    // Verify added
    let list = client.get_maintainers(&project_id);
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), maintainer);

    // Get project and verify maintainers field is populated
    let proj = client.get_project(&project_id).unwrap();
    assert_eq!(proj.maintainers.unwrap().len(), 1);

    // Owner removes maintainer
    client
        .mock_all_auths()
        .remove_maintainer(&project_id, &owner, &maintainer);

    // Verify removed
    let list = client.get_maintainers(&project_id);
    assert_eq!(list.len(), 0);
}

#[test]
fn test_maintainer_can_update_metadata() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    // ... rest of test (assume unchanged)
}
