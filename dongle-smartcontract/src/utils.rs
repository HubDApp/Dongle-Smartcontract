use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, String as SorobanString};

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
        env: &Env,
        caller: &Address,
        new_admin: &Address,
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
        value: &SorobanString,
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
    pub fn is_valid_ipfs_cid(cid: &SorobanString) -> bool {
        let len = cid.len();
        len >= 46 && len <= 100
    }

    /// Placeholder URL validator (always true for now).
    pub fn is_valid_url(_url: &SorobanString) -> bool {
        true
    }

    /// Returns the storage key as-is (identity helper).
    pub fn get_storage_key(data_key: StorageKey) -> StorageKey {
        data_key
    }

    /// Returns a clone of the input string (no-op sanitizer for now).
    pub fn sanitize_string(input: &SorobanString) -> SorobanString {
        input.clone()
    }

    /// Placeholder category validator (always true for now).
    pub fn is_valid_category(_category: &SorobanString) -> bool {
        true
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
