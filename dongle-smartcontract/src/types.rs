use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Review {
    pub reviewer:    Address,
    pub project_id:  u64,
    pub rating:      u32,
    pub comment_cid: Option<String>,
    pub created_at:  u64,
    pub updated_at:  u64,
}

#[contracttype]
#[derive(Clone, Debug, Default)]
pub struct ProjectAggregate {
    pub total_rating: u64,
    pub review_count: u64,
}