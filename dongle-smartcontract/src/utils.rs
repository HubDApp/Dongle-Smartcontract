use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, String};

pub struct Utils;

impl Utils {
    /// Returns the current ledger timestamp.
    pub fn get_current_timestamp(env: &Env) -> u64 {
        env.ledger().timestamp()
    }

    /// Checks if the given address is the stored admin.
    pub fn is_admin(env: &Env, address: &Address) -> bool {
        let admin: Option<Address> = env.storage().persistent().get(&StorageKey::Admin);
        match admin {
            Some(a) => a == *address,
            None => false,
        }
    }

    /// Sets a new admin. Caller must be the current admin.
    pub fn add_admin(
        _env: &Env,
        _caller: &Address,
        _new_admin: &Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        if !Self::is_admin(env, caller) {
            return Err(ContractError::UnauthorizedAdmin);
        }
        env.storage()
            .persistent()
            .set(&StorageKey::Admin, new_admin);
        Ok(())
    }

    /// Removes admin by clearing the stored admin. Caller must be current admin.
    pub fn remove_admin(
        env: &Env,
        caller: &Address,
        _admin_to_remove: &Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        if !Self::is_admin(env, caller) {
            return Err(ContractError::UnauthorizedAdmin);
        }
        env.storage().persistent().remove(&StorageKey::Admin);
        Ok(())
    }

    /// Validates a Soroban string is within length bounds.
    pub fn validate_string_length(
        value: &String,
        min_length: u32,
        max_length: u32,
    ) -> Result<(), ContractError> {
        let length = value.len();
        if length < min_length {
            return Err(ContractError::InvalidProjectName);
        }
        if length > max_length {
            return Err(ContractError::StringLengthExceeded);
        }
        Ok(())
    }

    /// Checks if a string looks like a valid IPFS CID by length.
    pub fn is_valid_ipfs_cid(cid: &String) -> bool {
        let len = cid.len();
        (46..=100).contains(&len)
    }

    /// Placeholder URL validator (always true for now).
    pub fn is_valid_url(_url: &String) -> bool {
        true
    }

    /// Returns the storage key as-is (identity helper).
    pub fn get_storage_key(data_key: StorageKey) -> StorageKey {
        data_key
    }

    /// Returns a clone of the input string (no-op sanitizer for now).
    pub fn sanitize_string(input: &String) -> String {
        input.clone()
    }

    /// Placeholder category validator (always true for now).
    pub fn is_valid_category(_category: &String) -> bool {
        true
    }

    pub fn create_event_data(_event_type: &str, _data: &str) -> String {
        todo!("Event data creation needs Env parameter for Soroban String construction")
    }

    /// Validates pagination parameters.
    pub fn validate_pagination(_start_id: u64, limit: u32) -> Result<(), ContractError> {
        const MAX_LIMIT: u32 = 100;
        if limit == 0 || limit > MAX_LIMIT {
            return Err(ContractError::InvalidProjectId);
        }
        Ok(())
    }
}
