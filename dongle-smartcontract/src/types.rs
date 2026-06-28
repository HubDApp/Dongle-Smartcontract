use soroban_sdk::{contracttype, Address, String, Vec};

/// Verification status of a project
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Rejected,
}

/// Parameters for registering a new project
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegistrationParams {
    pub owner: Address,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub license: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<String>,
    pub launch_timestamp: Option<u64>,
    pub bounty_url: Option<String>,
}

/// Parameters for updating an existing project
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdateParams {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub website: Option<String>,
    pub license: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<String>,
    pub launch_timestamp: Option<u64>,
    pub bounty_url: Option<String>,
    pub bounty_url_clear: bool,  // if true and bounty_url is None, clear existing bounty
}

/// Full project data stored on-chain
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub license: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<String>,
    pub launch_timestamp: Option<u64>,
    pub bounty_url: Option<String>,
    pub maintainers: Option<Vec<Address>>,
    pub archived: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub verification_status: VerificationStatus,
    pub verified_at: Option<u64>,
}

/// Slug index key for uniqueness check
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlugIndexKey {
    pub slug: String,
}

// Other types omitted...
