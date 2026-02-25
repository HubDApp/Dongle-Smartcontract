//! Tests for admin role management and access control

#[cfg(test)]
mod tests {
    use crate::DongleContract;
    use crate::DongleContractClient;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, String as SorobanString};

    fn setup(env: &Env) -> (DongleContractClient, Address) {
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        (client, admin)
    }

    #[test]
    fn test_initialize_contract_with_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        assert!(client.is_admin(&admin));
        let admins = client.list_admins();
        assert_eq!(admins.len(), 1);
        assert_eq!(admins.get(0).unwrap(), admin);
    }

    #[test]
    #[should_panic(expected = "Contract already initialized")]
    fn test_cannot_initialize_twice() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let admin2 = Address::generate(&env);
        client.mock_all_auths().initialize(&admin2);
    }

    #[test]
    fn test_admin_can_add_new_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let new_admin = Address::generate(&env);
        client.mock_all_auths().add_admin(&admin, &new_admin);

        assert!(client.is_admin(&new_admin));
        let admins = client.list_admins();
        assert_eq!(admins.len(), 2);
    }

    #[test]
    fn test_non_admin_cannot_add_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let non_admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        let result = client.try_add_admin(&non_admin, &new_admin);
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_add_existing_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let result = client.try_add_admin(&admin, &admin);
        assert!(result.is_err());
    }

    #[test]
    fn test_admin_can_remove_another_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let admin2 = Address::generate(&env);
        client.mock_all_auths().add_admin(&admin, &admin2);

        assert!(client.is_admin(&admin2));

        client.mock_all_auths().remove_admin(&admin, &admin2);

        assert!(!client.is_admin(&admin2));
        let admins = client.list_admins();
        assert_eq!(admins.len(), 1);
    }

    #[test]
    fn test_cannot_remove_last_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let result = client.try_remove_admin(&admin, &admin);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_admin_cannot_remove_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let admin2 = Address::generate(&env);
        client.mock_all_auths().add_admin(&admin, &admin2);

        let non_admin = Address::generate(&env);
        let result = client.try_remove_admin(&non_admin, &admin2);
        assert!(result.is_err());
    }

    #[test]
    fn test_only_admin_can_approve_verification() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let owner = Address::generate(&env);
        let project_id = client.mock_all_auths().register_project(
            &owner,
            &SorobanString::from_str(&env, "Test Project"),
            &SorobanString::from_str(&env, "A test project description that is long enough"),
            &SorobanString::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        client.mock_all_auths().request_verification(
            &project_id,
            &owner,
            &SorobanString::from_str(&env, "evidence_cid"),
        );

        // Admin can approve
        client
            .mock_all_auths()
            .approve_verification(&project_id, &admin);

        let verification = client.get_verification(&project_id).unwrap();
        assert_eq!(
            verification.status,
            crate::types::VerificationStatus::Verified
        );
    }

    #[test]
    fn test_non_admin_cannot_approve_verification() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let owner = Address::generate(&env);
        let project_id = client.mock_all_auths().register_project(
            &owner,
            &SorobanString::from_str(&env, "Test Project"),
            &SorobanString::from_str(&env, "A test project description that is long enough"),
            &SorobanString::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        client.mock_all_auths().request_verification(
            &project_id,
            &owner,
            &SorobanString::from_str(&env, "evidence_cid"),
        );

        let non_admin = Address::generate(&env);
        let result = client.try_approve_verification(&project_id, &non_admin);
        assert!(result.is_err());
    }

    #[test]
    fn test_only_admin_can_set_fee() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let treasury = Address::generate(&env);
        client
            .mock_all_auths()
            .set_fee(&admin, &None, &1000000, &treasury);

        let fee_config = client.get_fee_config();
        assert_eq!(fee_config.verification_fee, 1000000);
    }

    #[test]
    fn test_non_admin_cannot_set_fee() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let non_admin = Address::generate(&env);
        let treasury = Address::generate(&env);

        let result = client.try_set_fee(&non_admin, &None, &1000000, &treasury);
        assert!(result.is_err());
    }

    #[test]
    fn test_only_admin_can_set_treasury() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let treasury = Address::generate(&env);
        client.mock_all_auths().set_treasury(&admin, &treasury);

        let retrieved_treasury = client.get_treasury().unwrap();
        assert_eq!(retrieved_treasury, treasury);
    }

    #[test]
    fn test_non_admin_cannot_set_treasury() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        client.mock_all_auths().initialize(&admin);

        let non_admin = Address::generate(&env);
        let treasury = Address::generate(&env);

        let result = client.try_set_treasury(&non_admin, &treasury);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_admins_workflow() {
        let env = Env::default();
        let (client, admin1) = setup(&env);

        client.mock_all_auths().initialize(&admin1);

        // Add second admin
        let admin2 = Address::generate(&env);
        client.mock_all_auths().add_admin(&admin1, &admin2);

        // Add third admin
        let admin3 = Address::generate(&env);
        client.mock_all_auths().add_admin(&admin2, &admin3);

        // All three should be admins
        assert!(client.is_admin(&admin1));
        assert!(client.is_admin(&admin2));
        assert!(client.is_admin(&admin3));

        let admins = client.list_admins();
        assert_eq!(admins.len(), 3);

        // Admin3 can remove admin2
        client.mock_all_auths().remove_admin(&admin3, &admin2);
        assert!(!client.is_admin(&admin2));

        // Now only 2 admins
        let admins = client.list_admins();
        assert_eq!(admins.len(), 2);
    }
}
