use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{DisputeResolutionAction, DisputeStatus, DuplicateDispute};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_duplicate_dispute_lifecycle() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let creator = Address::generate(&env);

    let id1 = create_test_project(&client, &owner1, "OriginalProject");
    let id2 = create_test_project(&client, &owner2, "DuplicateProject");

    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Open dispute
    let dispute_id =
        client
            .mock_all_auths()
            .open_duplicate_dispute(&id2, &id1, &creator, &evidence_cid);

    assert_eq!(dispute_id, 1);

    // Get dispute
    let dispute = client.get_duplicate_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.id, 1);
    assert_eq!(dispute.project_id, id2);
    assert_eq!(dispute.original_project_id, id1);
    assert_eq!(dispute.creator, creator);
    assert_eq!(dispute.evidence_cid, evidence_cid);
    assert_eq!(dispute.status, DisputeStatus::Pending);

    // Get disputes for project
    let disputes = client.get_disputes_for_project(&id2);
    assert_eq!(disputes.len(), 1);
    assert_eq!(disputes.get(0).unwrap().id, 1);

    // Non-admin tries to resolve (should fail)
    let non_admin = Address::generate(&env);
    let result = client.mock_all_auths().try_resolve_duplicate_dispute(
        &1,
        &non_admin,
        &DisputeResolutionAction::Reject,
    );
    assert!(result.is_err());

    // Admin rejects dispute
    client
        .mock_all_auths()
        .resolve_duplicate_dispute(&1, &admin, &DisputeResolutionAction::Reject);

    let dispute = client.get_duplicate_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Rejected);
}

#[test]
fn test_resolve_dispute_archive() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let creator = Address::generate(&env);

    let id1 = create_test_project(&client, &owner1, "OriginalProject");
    let id2 = create_test_project(&client, &owner2, "DuplicateProject");

    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    let dispute_id =
        client
            .mock_all_auths()
            .open_duplicate_dispute(&id2, &id1, &creator, &evidence_cid);

    // Resolve by archiving duplicate project (id2)
    client.mock_all_auths().resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::ArchiveProject(id2),
    );

    let dispute = client.get_duplicate_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);

    // Check duplicate project is archived
    let project = client.get_project(&id2).unwrap();
    assert!(project.archived);
}

#[test]
fn test_resolve_dispute_link() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    let creator = Address::generate(&env);

    let id1 = create_test_project(&client, &owner1, "OriginalProject");
    let id2 = create_test_project(&client, &owner2, "DuplicateProject");

    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    let dispute_id =
        client
            .mock_all_auths()
            .open_duplicate_dispute(&id2, &id1, &creator, &evidence_cid);

    // Resolve by linking duplicates
    client.mock_all_auths().resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::LinkDuplicates,
    );

    let dispute = client.get_duplicate_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);

    // Check project links
    let links = client.get_linked_projects(&id2);
    assert_eq!(links.len(), 1);
    assert_eq!(links.get(0).unwrap(), id1);
}
