#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_collection_snapshots_and_comparison() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "AlphaSnap");
    let p2 = create_test_project(&client, &owner, "BetaSnap");
    let p3 = create_test_project(&client, &owner, "GammaSnap");

    let col_id = client.create_collection(
        &admin,
        &String::from_str(&env, "SnapshotCollection"),
        &String::from_str(&env, "Testing snapshots"),
    );

    // Add p1 and p2
    client.add_project_to_collection(&admin, &col_id, &p1);
    client.add_project_to_collection(&admin, &col_id, &p2);

    // Take snapshot 1
    let cid1 = String::from_str(&env, "bafybeigdyrzt5sfp7udm7ahu76uh7y26nf3efuylqabf3oclgtqy55fbkd");
    let snap1 = client.snapshot_collection(&admin, &col_id, &Some(cid1));
    assert_eq!(snap1.version, 1);
    assert_eq!(snap1.project_count, 2);

    // Remove p1 and add p3
    client.remove_project_from_collection(&admin, &col_id, &p1);
    client.add_project_to_collection(&admin, &col_id, &p3);

    // Take snapshot 2
    let cid2 = String::from_str(&env, "bafybeigdyrzt5sfp7udm7ahu76uh7y26nf3efuylqabf3oclgtqy55fbkd");
    let snap2 = client.snapshot_collection(&admin, &col_id, &Some(cid2));
    assert_eq!(snap2.version, 2);
    assert_eq!(snap2.project_count, 2);

    // Historical retrieval
    let retrieved_v1 = client.get_collection_snapshot(&col_id, &1).unwrap();
    assert_eq!(retrieved_v1.version, 1);
    assert!(retrieved_v1.project_ids.contains(p1));

    let latest = client.get_latest_collection_snapshot(&col_id).unwrap();
    assert_eq!(latest.version, 2);
    assert!(latest.project_ids.contains(p3));

    // List snapshots
    let all_snaps = client.list_collection_snapshots(&col_id, &0, &10);
    assert_eq!(all_snaps.len(), 2);

    // Compare snapshots
    let comparison = client.compare_collection_snapshots(&col_id, &1, &2);
    assert_eq!(comparison.version_a, 1);
    assert_eq!(comparison.version_b, 2);
    assert_eq!(comparison.added_projects.len(), 1);
    assert_eq!(comparison.added_projects.get(0).unwrap(), p3);
    assert_eq!(comparison.removed_projects.len(), 1);
    assert_eq!(comparison.removed_projects.get(0).unwrap(), p1);
    assert_eq!(comparison.unchanged_projects.len(), 1);
    assert_eq!(comparison.unchanged_projects.get(0).unwrap(), p2);
}
