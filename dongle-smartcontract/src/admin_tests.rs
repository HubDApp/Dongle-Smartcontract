//! Integration tests for admin role management and access control

#[cfg(test)]
mod tests {
    use crate::DongleContract;
    use crate::DongleContractClient;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    fn setup(env: &Env) -> (DongleContractClient, Address) {
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.mock_all_auths().initialize(&admin);
        (client, admin)
    }

    #[test]
    fn test_admin_initialization() {
        let env = Env::default();
        let (client, admin) = setup(&env);

        assert!(client.is_admin(&admin));
        assert_eq!(client.get_admin_count(), 1);

        let admin_list = client.get_admin_list();
        assert_eq!(admin_list.len(), 1);
        assert_eq!(admin_list.get(0).unwrap(), admin);
    }

    #[test]
    fn test_add_multiple_admins() {
        let env = Env::default();
        let (client, admin1) = setup(&env);
        let admin2 = Address::generate(&env);
        let admin3 = Address::generate(&env);

        client.mock_all_auths().add_admin(&admin1, &admin2);
        client.mock_all_auths().add_admin(&admin1, &admin3);

        assert!(client.is_admin(&admin1));
        assert!(client.is_admin(&admin2));
        assert!(client.is_admin(&admin3));
        assert_eq!(client.get_admin_count(), 3);
    }

    #[test]
    fn test_remove_admin_success() {
        let env = Env::default();
        let (client, admin1) = setup(&env);
        let admin2 = Address::generate(&env);

        client.mock_all_auths().add_admin(&admin1, &admin2);
        assert_eq!(client.get_admin_count(), 2);

        client.mock_all_auths().remove_admin(&admin1, &admin2);

        assert!(client.is_admin(&admin1));
        assert!(!client.is_admin(&admin2));
        assert_eq!(client.get_admin_count(), 1);
    }

    #[test]
    fn test_admin_can_set_fees() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        let treasury = Address::generate(&env);
        let fee_amount = 1000u128;

        client
            .mock_all_auths()
            .set_fee(&admin, &None, &fee_amount, &treasury);

        let config = client.get_fee_config();
        assert_eq!(config.verification_fee, fee_amount);
    }

    #[test]
    fn test_multiple_admins_can_perform_actions() {
        let env = Env::default();
        let (client, admin1) = setup(&env);
        let admin2 = Address::generate(&env);
        let treasury = Address::generate(&env);

        client.mock_all_auths().add_admin(&admin1, &admin2);

        // Both admins can set fees
        client
            .mock_all_auths()
            .set_fee(&admin1, &None, &1000u128, &treasury);
        client
            .mock_all_auths()
            .set_fee(&admin2, &None, &2000u128, &treasury);

        let config = client.get_fee_config();
        assert_eq!(config.verification_fee, 2000u128);
    }

    #[test]
    fn test_admin_list_updates_correctly() {
        let env = Env::default();
        let (client, admin1) = setup(&env);
        let admin2 = Address::generate(&env);
        let admin3 = Address::generate(&env);

        let list = client.get_admin_list();
        assert_eq!(list.len(), 1);

        client.mock_all_auths().add_admin(&admin1, &admin2);
        let list = client.get_admin_list();
        assert_eq!(list.len(), 2);

        client.mock_all_auths().add_admin(&admin1, &admin3);
        let list = client.get_admin_list();
        assert_eq!(list.len(), 3);

        client.mock_all_auths().remove_admin(&admin1, &admin2);
        let list = client.get_admin_list();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&admin1));
        assert!(list.contains(&admin3));
    }
}
