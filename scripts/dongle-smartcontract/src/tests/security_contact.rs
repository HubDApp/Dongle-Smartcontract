use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

const PROOF_CID: &str = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";

#[test]
fn update_security_contact_sets_unverified_contact() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "SecurityContactProject");
    let contact = String::from_str(&env, "security@example.com");

    let project = client.mock_all_auths().update_security_contact(
        &project_id,
        &owner,
        &Some(contact.clone()),
    );

    assert_eq!(project.security_contact, Some(contact.clone()));
    assert_eq!(project.security_contact_proof_cid, None);
    assert!(!project.security_contact_verified);

    let status = client.get_security_contact_status(&project_id);
    assert_eq!(status.contact, Some(contact));
    assert_eq!(status.proof_cid, None);
    assert!(!status.verified);
}

#[test]
fn submit_security_contact_proof_marks_contact_verified() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProofProject");
    let contact = String::from_str(&env, "https://example.com/.well-known/security.txt");
    let proof_cid = String::from_str(&env, PROOF_CID);

    client
        .mock_all_auths()
        .update_security_contact(&project_id, &owner, &Some(contact.clone()));
    let project =
        client
            .mock_all_auths()
            .submit_security_contact_proof(&project_id, &owner, &proof_cid);

    assert_eq!(project.security_contact, Some(contact.clone()));
    assert_eq!(project.security_contact_proof_cid, Some(proof_cid.clone()));
    assert!(project.security_contact_verified);

    let status = client.get_security_contact_status(&project_id);
    assert_eq!(status.contact, Some(contact));
    assert_eq!(status.proof_cid, Some(proof_cid));
    assert!(status.verified);
}

#[test]
fn changing_security_contact_clears_previous_proof() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ContactRotationProject");
    let first_contact = String::from_str(&env, "security@example.com");
    let second_contact = String::from_str(&env, "security-new@example.com");
    let proof_cid = String::from_str(&env, PROOF_CID);

    client
        .mock_all_auths()
        .update_security_contact(&project_id, &owner, &Some(first_contact));
    client
        .mock_all_auths()
        .submit_security_contact_proof(&project_id, &owner, &proof_cid);

    let project = client.mock_all_auths().update_security_contact(
        &project_id,
        &owner,
        &Some(second_contact.clone()),
    );

    assert_eq!(project.security_contact, Some(second_contact));
    assert_eq!(project.security_contact_proof_cid, None);
    assert!(!project.security_contact_verified);
}

#[test]
fn proof_submission_requires_contact_and_valid_cid() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProofValidationProject");
    let proof_cid = String::from_str(&env, PROOF_CID);

    let result =
        client
            .mock_all_auths()
            .try_submit_security_contact_proof(&project_id, &owner, &proof_cid);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData)));

    client.mock_all_auths().update_security_contact(
        &project_id,
        &owner,
        &Some(String::from_str(&env, "security@example.com")),
    );

    let bad_cid = String::from_str(&env, "not-a-cid");
    let result =
        client
            .mock_all_auths()
            .try_submit_security_contact_proof(&project_id, &owner, &bad_cid);
    assert_eq!(result, Err(Ok(ContractError::InvalidMetaCid)));
}

#[test]
fn unauthorized_user_cannot_update_contact_or_submit_proof() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ContactAuthProject");
    let contact = String::from_str(&env, "security@example.com");
    let proof_cid = String::from_str(&env, PROOF_CID);

    let result = client.mock_all_auths().try_update_security_contact(
        &project_id,
        &stranger,
        &Some(contact.clone()),
    );
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));

    client
        .mock_all_auths()
        .update_security_contact(&project_id, &owner, &Some(contact));
    let result = client.mock_all_auths().try_submit_security_contact_proof(
        &project_id,
        &stranger,
        &proof_cid,
    );
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}
