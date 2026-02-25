use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, String};

pub struct Utils;

impl Utils {
    pub fn get_current_timestamp(_env: &Env) -> u64 {
        0
    }

    pub fn is_admin(_env: &Env, _address: &Address) -> bool {
        false
    }

    pub fn add_admin(_env: &Env, _caller: &Address, _new_admin: &Address) -> Result<(), Error> {
        todo!("Admin addition logic not implemented")
    }

    pub fn remove_admin(
        _env: &Env,
        _caller: &Address,
        _admin_to_remove: &Address,
    ) -> Result<(), Error> {
        todo!("Admin removal logic not implemented")
    }

    pub fn validate_string_length(
        value: &String,
        min_length: u32,
        max_length: u32,
        field_name: &str,
    ) -> Result<(), Error> {
        let length = value.len();

        if length < min_length || length > max_length {
            match field_name {
                "name" => Err(Error::InvalidProjectName),
                "description" => Err(Error::InvalidProjectDescription),
                _ => Err(Error::StringLengthExceeded),
            }
        } else {
            Ok(())
        }
    }

    pub fn is_valid_ipfs_cid(cid: &String) -> bool {
        let len = cid.len();
        len >= 46 && len <= 100
    }

    pub fn is_valid_url(_url: &String) -> bool {
        true
    }

    pub fn get_storage_key(data_key: StorageKey) -> StorageKey {
        data_key
    }

    pub fn sanitize_string(input: &String) -> String {
        input.clone()
    }

    pub fn is_valid_category(_category: &String) -> bool {
        true
    }

    pub fn create_event_data(_event_type: &str, _data: &str) -> String {
        todo!("Event data creation needs Env parameter for Soroban String construction")
    }

    pub fn validate_pagination(_start_id: u64, limit: u32) -> Result<(), Error> {
        const MAX_LIMIT: u32 = 100;

        if limit == 0 || limit > MAX_LIMIT {
            return Err(Error::StringLengthExceeded);
        }

        Ok(())
    }
}
