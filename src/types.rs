//! Contract data types. All types used in storage or events must be [contracttype].

use soroban_sdk::{contracttype, Address, String};

/// Status of a project's verification request.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VerificationStatus {
    Pending = 0,
    Verified = 1,
    Rejected = 2,
}

/// On-chain project metadata.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub description: String,
    /// Validated category — one of: DeFi, NFT, Gaming, DAO, Tools.
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// A single review for a project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,
    pub comment_cid: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Verification request and outcome.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRecord {
    pub project_id: u64,
    pub requester: Address,
    pub evidence_cid: String,
    pub status: VerificationStatus,
    pub requested_at: u64,
    pub decided_at: Option<u64>,
}

/// Fee configuration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub token: Option<Address>,
    pub amount: u128,
    pub treasury: Address,
}

/// `DataKey` is an alias for `StorageKey` (Issue #8: storage key enum must be named `DataKey`).
/// The canonical definition with `#[contracttype]` lives in `crate::storage_keys`
/// to avoid duplicate trait implementations.
pub use crate::storage_keys::StorageKey as DataKey;
