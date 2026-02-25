use crate::errors::ContractError;
use soroban_sdk::{String};

pub struct Utils;

impl Utils {
    pub fn validate_string_length(
        value: &String,
        min_length: u32,
        max_length: u32,
        _field_name: &str,
    ) -> Result<(), ContractError> {
        let length = value.len();

        if length < min_length || length > max_length {
            return Err(ContractError::InvalidProjectData);
        }
        
        Ok(())
    }

    pub fn is_valid_ipfs_cid(cid: &String) -> bool {
        let len = cid.len();
        (46..=100).contains(&len)
    }

    pub fn is_valid_url(_url: &String) -> bool {
        true
    }

    pub fn sanitize_string(input: &String) -> String {
        input.clone()
    }

    pub fn is_valid_category(_category: &String) -> bool {
        true
    }

    pub fn validate_pagination(_start_id: u64, limit: u32) -> Result<(), ContractError> {
        const MAX_LIMIT: u32 = 100;

        if limit == 0 || limit > MAX_LIMIT {
            return Err(ContractError::InvalidProjectData);
        }

        Ok(())
    }
}
