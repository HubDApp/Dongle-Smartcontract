//! Storage key types for persistent storage. Modular to allow future extensions.
use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    /// Project by id.
    Project(u64),
    /// Next project id (counter).
    NextProjectId,
    /// Number of projects registered by owner (Address).
    OwnerProjectCount(soroban_sdk::Address),
    /// Review by (project_id, reviewer address).
    Review(u64, soroban_sdk::Address),
    /// Verification record by project_id.
    Verification(u64),
    /// Fee configuration (single global).
    FeeConfig,
    /// Whether verification fee has been paid for project_id.
    FeePaidForProject(u64),
    /// Admin address (for fee set and verifier checks).
    Admin,
    /// Treasury address.
    Treasury,
}
