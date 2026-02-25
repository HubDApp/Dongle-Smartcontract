//! Integration tests for complete admin workflows

#[cfg(test)]
mod tests {
    use crate::types::VerificationStatus;
    use crate::DongleContract;
    use crate::DongleContractClient;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, String as SorobanString};

    fn setup_with_admin(env: &Env) -> (DongleContractClient, Address, Address) {
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let user = Address::generate(env);

        client.mock_all_auths().initialize(&admin);

        (client, admin, user)
    }

    #[test]
    fn test_complete_verification_approval_workflow() {
        let env = Env::default();
        let (client, admin, project_owner) = setup_with_admin(&env);

        let project_id = client.mock_all_auths().register_project(
            &project_owner,
            &SorobanString::from_str(&env, "DeFi Protocol"),
            &SorobanString::from_str(&env, "A decentralized finance protocol"),
            &SorobanString::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        client.mock_all_auths().request_verification(
            &project_id,
            &project_owner,
            &SorobanString::from_str(&env, "ipfs://evidence"),
        );

        let verification = client.get_verification(&project_id).unwrap();
        assert_eq!(verification.status, VerificationStatus::Pending);

        client
            .mock_all_auths()
            .approve_verification(&project_id, &admin);

        let verification = client.get_verification(&project_id).unwrap();
        assert_eq!(verification.status, VerificationStatus::Verified);
    }

    #[test]
    fn test_complete_verification_rejection_workflow() {
        let env = Env::default();
        let (client, admin, project_owner) = setup_with_admin(&env);

        let project_id = client.mock_all_auths().register_project(
            &project_owner,
            &SorobanString::from_str(&env, "Test Project"),
            &SorobanString::from_str(&env, "A test project description"),
            &SorobanString::from_str(&env, "Other"),
            &None,
            &None,
            &None,
        );

        client.mock_all_auths().request_verification(
            &project_id,
            &project_owner,
            &SorobanString::from_str(&env, "ipfs://evidence"),
        );

        client
            .mock_all_auths()
            .reject_verification(&project_id, &admin);

        let verification = client.get_verification(&project_id).unwrap();
        assert_eq!(verification.status, VerificationStatus::Rejected);
    }

    #[test]
    fn test_non_owner_cannot_request_verification() {
        let env = Env::default();
        let (client, _admin, project_owner) = setup_with_admin(&env);

        let project_id = client.mock_all_auths().register_project(
            &project_owner,
            &SorobanString::from_str(&env, "Test Project"),
            &SorobanString::from_str(&env, "Description"),
            &SorobanString::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        let non_owner = Address::generate(&env);
        let result = client.try_request_verification(
            &project_id,
            &non_owner,
            &SorobanString::from_str(&env, "ipfs://evidence"),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_approve_already_verified_project() {
        let env = Env::default();
        let (client, admin, project_owner) = setup_with_admin(&env);

        let project_id = client.mock_all_auths().register_project(
            &project_owner,
            &SorobanString::from_str(&env, "Test Project"),
            &SorobanString::from_str(&env, "Description"),
            &SorobanString::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        client.mock_all_auths().request_verification(
            &project_id,
            &project_owner,
            &SorobanString::from_str(&env, "ipfs://evidence"),
        );

        client
            .mock_all_auths()
            .approve_verification(&project_id, &admin);

        let result = client.try_approve_verification(&project_id, &admin);
        assert!(result.is_err());
    }

    #[test]
    fn test_fee_management_workflow() {
        let env = Env::default();
        let (client, admin, _user) = setup_with_admin(&env);

        let treasury = Address::generate(&env);

        client
            .mock_all_auths()
            .set_fee(&admin, &None, &1000000, &treasury);

        let fee_config = client.get_fee_config();
        assert_eq!(fee_config.verification_fee, 1000000);

        client.mock_all_auths().set_treasury(&admin, &treasury);
        let retrieved_treasury = client.get_treasury().unwrap();
        assert_eq!(retrieved_treasury, treasury);
    }

    #[test]
    fn test_multi_admin_verification_workflow() {
        let env = Env::default();
        let (client, admin1, project_owner) = setup_with_admin(&env);

        let admin2 = Address::generate(&env);
        client.mock_all_auths().add_admin(&admin1, &admin2);

        let project_id = client.mock_all_auths().register_project(
            &project_owner,
            &SorobanString::from_str(&env, "Multi Admin Test"),
            &SorobanString::from_str(&env, "Testing with multiple admins"),
            &SorobanString::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        client.mock_all_auths().request_verification(
            &project_id,
            &project_owner,
            &SorobanString::from_str(&env, "ipfs://evidence"),
        );

        client
            .mock_all_auths()
            .approve_verification(&project_id, &admin2);

        let verification = client.get_verification(&project_id).unwrap();
        assert_eq!(verification.status, VerificationStatus::Verified);
    }

    #[test]
    fn test_list_pending_verifications() {
        let env = Env::default();
        let (client, admin, owner1) = setup_with_admin(&env);

        let owner2 = Address::generate(&env);

        let project1 = client.mock_all_auths().register_project(
            &owner1,
            &SorobanString::from_str(&env, "Project 1"),
            &SorobanString::from_str(&env, "Description 1"),
            &SorobanString::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        let project2 = client.mock_all_auths().register_project(
            &owner2,
            &SorobanString::from_str(&env, "Project 2"),
            &SorobanString::from_str(&env, "Description 2"),
            &SorobanString::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );

        client.mock_all_auths().request_verification(
            &project1,
            &owner1,
            &SorobanString::from_str(&env, "evidence1"),
        );

        client.mock_all_auths().request_verification(
            &project2,
            &owner2,
            &SorobanString::from_str(&env, "evidence2"),
        );

        // Approve one
        client
            .mock_all_auths()
            .approve_verification(&project1, &admin);

        // List pending - should only find project2
        let pending = client
            .mock_all_auths()
            .list_pending_verifications(&admin, &1, &5);

        assert_eq!(pending.len(), 1);
        let (pid, record) = pending.get(0).unwrap();
        assert_eq!(pid, project2);
        assert_eq!(record.status, VerificationStatus::Pending);
    }

    #[test]
    fn test_admin_rotation_maintains_access_control() {
        let env = Env::default();
        let (client, admin1, project_owner) = setup_with_admin(&env);

        let admin2 = Address::generate(&env);
        client.mock_all_auths().add_admin(&admin1, &admin2);

        let project_id = client.mock_all_auths().register_project(
            &project_owner,
            &SorobanString::from_str(&env, "Test Project"),
            &SorobanString::from_str(&env, "Description"),
            &SorobanString::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        client.mock_all_auths().request_verification(
            &project_id,
            &project_owner,
            &SorobanString::from_str(&env, "evidence"),
        );

        client.mock_all_auths().remove_admin(&admin2, &admin1);

        assert!(!client.is_admin(&admin1));
        assert!(client.is_admin(&admin2));

        client
            .mock_all_auths()
            .approve_verification(&project_id, &admin2);

        let verification = client.get_verification(&project_id).unwrap();
        assert_eq!(verification.status, VerificationStatus::Verified);
    }
}
