//! Contract data types. All types used in storage or events must be [contracttype].
//! Uses soroban_sdk::String for Val/testutils compatibility.

use soroban_sdk::contracttype;

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
    pub owner: soroban_sdk::Address,
    pub name: soroban_sdk::String,
    pub description: soroban_sdk::String,
    pub category: soroban_sdk::String,
    pub website: Option<soroban_sdk::String>,
    pub logo_cid: Option<soroban_sdk::String>,
    pub metadata_cid: Option<soroban_sdk::String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// A single review for a project. Rating is 1..=5 (u32 for Soroban Val compatibility).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    pub project_id: u64,
    pub reviewer: soroban_sdk::Address,
    pub rating: u32,
    pub comment_cid: Option<soroban_sdk::String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Verification request and outcome.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRecord {
    pub project_id: u64,
    pub requester: soroban_sdk::Address,
    pub evidence_cid: soroban_sdk::String,
    pub status: VerificationStatus,
    pub requested_at: u64,
    pub decided_at: Option<u64>,
}

/// Fee configuration (admin-set).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub token: Option<soroban_sdk::Address>,
    pub amount: u128,
    pub treasury: soroban_sdk::Address,
}
