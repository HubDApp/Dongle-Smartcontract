use crate::errors::ContractError;
use crate::types::DataKey;
use soroban_sdk::{Address, Env, String};

pub struct Utils;

impl Utils {
    pub fn get_current_timestamp(env: &Env) -> u64 {
        0
    }

    pub fn is_admin(env: &Env, address: &Address) -> bool {
        false
    }

    pub fn add_admin(
        env: &Env,
        caller: &Address,
        new_admin: &Address,
    ) -> Result<(), ContractError> {
        todo!("Admin addition logic not implemented")
    }

    pub fn remove_admin(
        env: &Env,
        caller: &Address,
        admin_to_remove: &Address,
    ) -> Result<(), ContractError> {
        todo!("Admin removal logic not implemented")
    }

    pub fn validate_string_length(
        value: &String,
        min_length: u32,
        max_length: u32,
        field_name: &str,
    ) -> Result<(), ContractError> {
        let length = value.len();

        if length < min_length || length > max_length {
            match field_name {
                "name" => Err(ContractError::ProjectNameTooLong),
                "description" => Err(ContractError::ProjectDescriptionTooLong),
                _ => Err(ContractError::InvalidProjectData),
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

    pub fn get_storage_key(data_key: DataKey) -> DataKey {
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

    pub fn validate_pagination(start_id: u64, limit: u32) -> Result<(), ContractError> {
        const MAX_LIMIT: u32 = 100;

        if limit == 0 || limit > MAX_LIMIT {
            return Err(ContractError::InvalidProjectData);
        }

        Ok(())
    }
}
