use alloc::vec::Vec;
use soroban_sdk::{Env, String};
use crate::errors::ContractError;

extern crate alloc;

pub fn validate_project_name(name: &String) -> Result<(), ContractError> {
    let name_bytes: Vec<u8> = name.to_alloc_vec();
    
    // 1. Non-empty
    if name_bytes.is_empty() {
        return Err(ContractError::EmptyProjectName);
    }
    
    // 2. Max length of 50 characters
    if name_bytes.len() > 50 {
        return Err(ContractError::InvalidProjectNameLength);
    }
    
    // 3. Not only whitespace (and check allowed chars)
    let mut has_non_whitespace = false;
    
    for byte in name_bytes {
        // Allowed characters: letters (a-z, A-Z), numbers (0-9), underscores (_), and hyphens (-)
        // We also allow spaces ( ) but the whole string can't be only spaces.
        let is_letter = (byte >= b'a' && byte <= b'z') || (byte >= b'A' && byte <= b'Z');
        let is_number = byte >= b'0' && byte <= b'9';
        let is_allowed_symbol = byte == b'_' || byte == b'-';
        let is_space = byte == b' ';
        
        if is_letter || is_number || is_allowed_symbol {
            has_non_whitespace = true;
        } else if !is_space {
            // Found a character that is strictly prohibited
            return Err(ContractError::InvalidProjectNameFormat);
        }
    }
    
    if !has_non_whitespace {
        // All characters were spaces
        return Err(ContractError::EmptyProjectName);
    }
    
    Ok(())
}
