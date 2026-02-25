//! Admin role management and access control
//!
//! This module provides functionality for managing admin roles and enforcing
//! access control across privileged contract operations.

use crate::errors::ContractError;
use crate::events::{publish_admin_added_event, publish_admin_removed_event};
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, Vec};

pub struct AdminManager;

impl AdminManager {
    /// Initialize the contract with the first admin
    pub fn initialize(env: &Env, admin: Address) {
        // Don't require auth during initialization - this is typically called once during contract deployment

        // Set the admin in storage
        env.storage()
            .persistent()
            .set(&StorageKey::Admin(admin.clone()), &true);

        // Initialize admin list
        let mut admins = Vec::new(env);
        admins.push_back(admin.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::AdminList, &admins);

        publish_admin_added_event(env, admin);
    }

    /// Add a new admin (only callable by existing admins)
    pub fn add_admin(env: &Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        caller.require_auth();

        // Verify caller is an admin
        Self::require_admin(env, &caller)?;

        // Check if already an admin
        if Self::is_admin(env, &new_admin) {
            return Ok(()); // Already an admin, no-op
        }

        // Add to admin mapping
        env.storage()
            .persistent()
            .set(&StorageKey::Admin(new_admin.clone()), &true);

        // Add to admin list
        let mut admins = Self::get_admin_list(env);
        admins.push_back(new_admin.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::AdminList, &admins);

        publish_admin_added_event(env, new_admin);

        Ok(())
    }

    /// Remove an admin (only callable by existing admins)
    pub fn remove_admin(
        env: &Env,
        caller: Address,
        admin_to_remove: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        // Verify caller is an admin
        Self::require_admin(env, &caller)?;

        // Prevent removing the last admin
        let admins = Self::get_admin_list(env);
        if admins.len() <= 1 {
            return Err(ContractError::CannotRemoveLastAdmin);
        }

        // Check if the address is actually an admin
        if !Self::is_admin(env, &admin_to_remove) {
            return Err(ContractError::AdminNotFound);
        }

        // Remove from admin mapping
        env.storage()
            .persistent()
            .remove(&StorageKey::Admin(admin_to_remove.clone()));

        // Remove from admin list
        let mut new_admins = Vec::new(env);
        for admin in admins.iter() {
            if admin != admin_to_remove {
                new_admins.push_back(admin);
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::AdminList, &new_admins);

        publish_admin_removed_event(env, admin_to_remove);

        Ok(())
    }

    /// Check if an address is an admin
    pub fn is_admin(env: &Env, address: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::Admin(address.clone()))
            .unwrap_or(false)
    }

    /// Require that the caller is an admin, otherwise return an error
    pub fn require_admin(env: &Env, address: &Address) -> Result<(), ContractError> {
        if Self::is_admin(env, address) {
            Ok(())
        } else {
            Err(ContractError::AdminOnly)
        }
    }

    /// Get the list of all admins
    pub fn get_admin_list(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&StorageKey::AdminList)
            .unwrap_or(Vec::new(env))
    }

    /// Get the count of admins
    pub fn get_admin_count(env: &Env) -> u32 {
        Self::get_admin_list(env).len()
    }
}

#[cfg(test)]
mod tests {
    use crate::DongleContract;
    use crate::DongleContractClient;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_initialize_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.mock_all_auths().initialize(&admin);

        assert!(client.is_admin(&admin));
        assert_eq!(client.get_admin_count(), 1);
    }

    #[test]
    fn test_add_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        client.mock_all_auths().initialize(&admin1);
        client.mock_all_auths().add_admin(&admin1, &admin2);

        assert!(client.is_admin(&admin2));
        assert_eq!(client.get_admin_count(), 2);
    }

    #[test]
    fn test_add_admin_unauthorized() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        client.mock_all_auths().initialize(&admin);
        client.mock_all_auths().add_admin(&non_admin, &new_admin);

        // The new admin should not be added
        assert!(!client.is_admin(&new_admin));
    }

    #[test]
    fn test_remove_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        client.mock_all_auths().initialize(&admin1);
        client.mock_all_auths().add_admin(&admin1, &admin2);
        client.mock_all_auths().remove_admin(&admin1, &admin2);

        assert!(!client.is_admin(&admin2));
        assert_eq!(client.get_admin_count(), 1);
    }

    #[test]
    fn test_cannot_remove_last_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.mock_all_auths().initialize(&admin);
        client.mock_all_auths().remove_admin(&admin, &admin);

        // Should still be admin
        assert!(client.is_admin(&admin));
    }

    #[test]
    fn test_remove_non_existent_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);
        let another_admin = Address::generate(&env);

        client.mock_all_auths().initialize(&admin);
        client.mock_all_auths().add_admin(&admin, &another_admin);
        client.mock_all_auths().remove_admin(&admin, &non_admin);

        // another_admin should still be there
        assert!(client.is_admin(&another_admin));
    }

    #[test]
    fn test_get_admin_list() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let admin3 = Address::generate(&env);

        client.mock_all_auths().initialize(&admin1);
        client.mock_all_auths().add_admin(&admin1, &admin2);
        client.mock_all_auths().add_admin(&admin1, &admin3);

        let admins = client.get_admin_list();
        assert_eq!(admins.len(), 3);
        assert!(admins.contains(&admin1));
        assert!(admins.contains(&admin2));
        assert!(admins.contains(&admin3));
    }
}
