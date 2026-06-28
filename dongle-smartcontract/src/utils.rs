use soroban_sdk::{Address, Env, String};

use crate::storage_keys::StorageKey;
use crate::types::Project;

/// Check if contract is initialized
pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&StorageKey::Initialized)
}

/// Validate slug format (alphanumeric and hyphens, no leading/trailing hyphens, length 3-50)
pub fn is_valid_slug(slug: &String) -> bool {
    let bytes = slug.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    for (i, &b) in bytes.iter().enumerate() {
        if !b.is_ascii_alphanumeric() && b != b'-' {
            return false;
        }
        if b == b'-' && (i == 0 || i == bytes.len() - 1) {
            return false;
        }
    }
    true
}

/// Validate category against allowed list (simplified)
pub fn is_valid_category(category: &String) -> bool {
    let allowed_categories = [
        "DeFi",
        "NFT",
        "Infrastructure",
        "Social",
        "Gaming",
        "Utility",
        "Other",
    ];
    allowed_categories.contains(&category.as_str())
}

/// Check if address is a maintainer of the project
pub fn is_maintainer(env: &Env, project: &Project, address: &Address) -> bool {
    if let Some(ref maintainers) = project.maintainers {
        maintainers.contains(address)
    } else {
        false
    }
}
