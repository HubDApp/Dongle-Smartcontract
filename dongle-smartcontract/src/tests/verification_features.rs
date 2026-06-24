use crate::types::VerificationStatus;
use crate::tests::fixtures::{setup_contract, create_test_project};
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String};

#[test]
fn test_verification_delay() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // Register a project. This sets the project's created_at time to env.ledger().timestamp().
    let project_id = create_test_project(&client, &owner, "ProjectAgeTest");

    // Let's set the minimum project age to 3600 seconds (1 hour)
    client.mock_all_auths().set_min_project_age(&admin, &3600);
    assert_eq!(client.get_min_project_age(), 3600);

    // Since ledger time is 0 and min age is 3600, requesting verification right now should fail with ProjectTooYoung.
    let evidence = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");
    let result = client.mock_all_auths().try_request_verification(&project_id, &owner, &evidence);
    assert!(result.is_err());

    // Advance ledger time by 3600 seconds
    env.ledger().set_timestamp(3600);

    // Now requesting verification should succeed
    let result = client.mock_all_auths().try_request_verification(&project_id, &owner, &evidence);
    assert!(result.is_ok());
}

#[test]
fn test_verification_delay_zero() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ProjectAgeTestZero");

    // Set min project age to 0
    client.mock_all_auths().set_min_project_age(&admin, &0);
    assert_eq!(client.get_min_project_age(), 0);

    // Requesting verification immediately should succeed
    let evidence = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");
    let result = client.mock_all_auths().try_request_verification(&project_id, &owner, &evidence);
    assert!(result.is_ok());
}

#[test]
fn test_verification_expiry_and_duration() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ProjectExpiryTest");

    // Set min project age to 0 to bypass age check
    client.mock_all_auths().set_min_project_age(&admin, &0);

    // Set verification validity duration to 1000 seconds
    client.mock_all_auths().set_verification_duration(&admin, &1000);
    assert_eq!(client.get_verification_duration(), 1000);

    // Request verification
    let evidence = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");
    client.mock_all_auths().request_verification(&project_id, &owner, &evidence);

    // Initially status is Pending
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);

    // Approve verification at timestamp 100
    env.ledger().set_timestamp(100);
    client.mock_all_auths().approve_verification(&project_id, &admin);

    // Check project status is Verified
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);

    // Check verification record's expires_at is 1100 (100 + 1000)
    let record = client.get_verification(&project_id);
    assert_eq!(record.expires_at, 1100);
    assert_eq!(record.status, VerificationStatus::Verified);

    // Check if it is expired at timestamp 500 (should be false)
    env.ledger().set_timestamp(500);
    assert!(!client.is_verification_expired(&project_id));

    // Check if it is expired at timestamp 1200 (should be true)
    env.ledger().set_timestamp(1200);
    assert!(client.is_verification_expired(&project_id));
}
