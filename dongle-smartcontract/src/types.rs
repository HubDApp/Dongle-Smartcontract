use soroban_sdk::{contracttype, Address, Map, String, Vec, Bool};

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
    pub social_links: Option<Vec<SocialLink>>,
    pub launch_timestamp: Option<u64>,
    pub bounty_url: Option<String>,
    pub bounty_cid: Option<String>,
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
    pub license: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub tags: Option<Vec<String>>,
    pub social_links: Option<Vec<SocialLink>>,
    pub launch_timestamp: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
    pub status: ProjectStatus,
    pub maintainers: Option<Vec<Address>>,
    pub archived: bool,
    pub featured: bool,
    pub verification_status: VerificationStatus,
    pub bounty_url: Option<String>,
    pub bounty_cid: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SocialLink {
    pub platform: String,
    pub url: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectStatus {
    Active,
    Archived,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Rejected,
}

pub fn validate_bounty_url(url: &String) -> bool {
    let s: &str = url.as_ref();
    s.starts_with("http://") || s.starts_with("https://")
}

pub fn validate_bounty_cid(cid: &String) -> bool {
    let s: &str = cid.as_ref();
    // Basic IPFS CID v0 (Qm...) or v1 (baf...)
    s.len() >= 46 && (s.starts_with("Qm") || s.starts_with("baf"))
}
