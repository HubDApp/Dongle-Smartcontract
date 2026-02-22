use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    // Rating aggregate fields
    pub rating_sum: u64,      // Sum of all ratings (scaled by 100 for precision)
    pub review_count: u32,    // Number of active reviews
    pub average_rating: u32,  // Cached average (scaled by 100, e.g., 450 = 4.50)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,          // Rating value 1-5
    pub comment_cid: Option<String>,
    pub timestamp: u64,
}
