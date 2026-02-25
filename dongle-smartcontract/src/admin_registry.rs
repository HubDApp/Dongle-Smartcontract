//! Admin role management with add/remove capabilities and access control.

use crate::errors::ContractError;
use crate::events::{publish_admin_added_event, publish_admin_removed_event};
use crate::types::DataKey;
use soroban_sdk::{Address, Env, Vec};

pub struct AdminRegistry;

impl AdminRegistry {
    /// Initialize the contract with the first admin
    pub fn initialize(env: &Env, admin: Address) {
        admin.require_auth();

        // Check if already initialized
        if env
            .storage()
            .persistent()
            .get::<DataKey, bool>(&DataKey::AdminInitialized)
            .unwrap_or(false)
        {
            panic!("Contract already initialized");
        }

        // Mark as initialized
        env.storage()
            .persistent()
            .set(&DataKey::AdminInitialized, &true);

        // Set admin
        env.storage()
            .persistent()
            .set(&DataKey::Admin(admin.clone()), &true);

        // Initialize admin list
        let mut admin_list = Vec::new(env);
        admin_list.push_back(admin.clone());
        env.storage()
            .persistent()
            .set(&DataKey::AdminList, &admin_list);
        env.storage().persistent().set(&DataKey::AdminCount, &1u32);

        publish_admin_added_event(env, admin.clone(), admin);
    }

    /// Add a new admin (only callable by existing admins)
    pub fn add_admin(env: &Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        caller.require_auth();

        // Verify caller is an admin
        if !Self::is_admin(env, &caller) {
            return Err(ContractError::AdminOnly);
        }

        // Check if already an admin
        if Self::is_admin(env, &new_admin) {
            return Err(ContractError::InvalidProjectData);
        }

        // Add to admin mapping
        env.storage()
            .persistent()
            .set(&DataKey::Admin(new_admin.clone()), &true);

        // Update admin list
        let mut admin_list = Self::list_admins(env);
        admin_list.push_back(new_admin.clone());
        env.storage()
            .persistent()
            .set(&DataKey::AdminList, &admin_list);

        // Update count
        let count = Self::get_admin_count(env);
        env.storage()
            .persistent()
            .set(&DataKey::AdminCount, &(count + 1));

        publish_admin_added_event(env, caller, new_admin);

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
        if !Self::is_admin(env, &caller) {
            return Err(ContractError::AdminOnly);
        }

        // Check if target is actually an admin
        if !Self::is_admin(env, &admin_to_remove) {
            return Err(ContractError::InvalidProjectData);
        }

        // Prevent removing the last admin
        if Self::get_admin_count(env) <= 1 {
            return Err(ContractError::InvalidProjectData);
        }

        // Remove from admin mapping
        env.storage()
            .persistent()
            .remove(&DataKey::Admin(admin_to_remove.clone()));

        // Update admin list
        let admin_list = Self::list_admins(env);
        let mut new_list = Vec::new(env);
        for i in 0..admin_list.len() {
            let addr = admin_list.get(i).unwrap();
            if addr != admin_to_remove {
                new_list.push_back(addr);
            }
        }
        env.storage()
            .persistent()
            .set(&DataKey::AdminList, &new_list);

        // Update count
        let count = Self::get_admin_count(env);
        env.storage()
            .persistent()
            .set(&DataKey::AdminCount, &(count - 1));

        publish_admin_removed_event(env, caller, admin_to_remove);

        Ok(())
    }

    /// Check if an address is an admin
    pub fn is_admin(env: &Env, address: &Address) -> bool {
        env.storage()
            .persistent()
            .get::<DataKey, bool>(&DataKey::Admin(address.clone()))
            .unwrap_or(false)
    }

    /// Require that the caller is an admin, panic otherwise
    pub fn require_admin(env: &Env, address: &Address) -> Result<(), ContractError> {
        if !Self::is_admin(env, address) {
            return Err(ContractError::AdminOnly);
        }
        Ok(())
    }

    /// Get count of admins (for preventing removal of last admin)
    fn get_admin_count(env: &Env) -> u32 {
        env.storage()
            .persistent()
            .get::<DataKey, u32>(&DataKey::AdminCount)
            .unwrap_or(0)
    }

    /// List all admins (for administrative purposes)
    pub fn list_admins(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get::<DataKey, Vec<Address>>(&DataKey::AdminList)
            .unwrap_or(Vec::new(env))
    }
}
