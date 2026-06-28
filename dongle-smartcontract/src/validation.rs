use soroban_sdk::String;

use crate::errors::ContractError;

/// Validate a URL (must start with http:// or https://, and be non-empty after trimming).
pub fn validate_bounty_url(url: &String) -> Result<(), ContractError> {
    let s = url.to_string();
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(ContractError::InvalidBountyUrl);
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(ContractError::InvalidBountyUrl);
    }
    // Optional: more thorough validation (e.g., check for valid characters after scheme)
    Ok(())
}

/// Validate a CID (IPFS CIDv0 or CIDv1).
/// CIDv0: starts with "Qm" and is base58btc encoded, length 46.
/// CIDv1: starts with "b" or "f", etc. We only accept CIDv0 and CIDv1 base32.
pub fn validate_bounty_cid(cid: &String) -> Result<(), ContractError> {
    let s = cid.to_string();
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(ContractError::InvalidBountyCid);
    }
    // Accept CIDv0 (starts with "Qm", length 46, valid base58btc characters)
    if trimmed.starts_with("Qm") && trimmed.len() == 46 {
        // Rough check: all characters are base58btc alphanumeric (no '0', 'O', 'I', 'l')
        if trimmed.chars().all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l') {
            return Ok(());
        }
    }
    // Accept CIDv1 base32 (starts with "b" followed by 59 characters, all lowercase letters or digits)
    if trimmed.starts_with("b") && trimmed.len() == 59 {
        let rest = &trimmed[1..];
        if rest.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return Ok(());
        }
    }
    // Accept CIDv1 base36 (starts with "k" or "K", but not common) - skip for simplicity
    Err(ContractError::InvalidBountyCid)
}
