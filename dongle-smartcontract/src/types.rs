use soroban_sdk::{contracttype, Address, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegistrationParams {
    pub owner: Address,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<Vec<String>>,
    pub launch_timestamp: Option<u64>,
    pub license: Option<String>,
    pub bounty_url: Option<String>,
    pub bounty_cid: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdateParams {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub website: Option<Option<String>>,
    pub logo_cid: Option<Option<String>>,
    pub metadata_cid: Option<Option<String>>,
    pub tags: Option<Option<Vec<String>>>,
    pub social_links: Option<Option<Vec<String>>>,
    pub launch_timestamp: Option<Option<u64>>,
    pub license: Option<Option<String>>,
    pub bounty_url: Option<Option<String>>,
    pub bounty_cid: Option<Option<String>>,
}

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
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<Vec<String>>,
    pub launch_timestamp: Option<u64>,
    pub license: Option<String>,
    pub bounty_url: Option<String>,
    pub bounty_cid: Option<String>,
    pub archived: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub maintainers: Option<Vec<Address>>,
    pub verification_status: Option<u32>,
    // ... other fields
}
