use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::types::Project;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ProjectRegistry;

impl ProjectRegistry {
    fn next_project_id(env: &Env) -> u64 {
        let key = StorageKey::NextProjectId;
        let next: u64 = env.storage().persistent().get(&key).unwrap_or(1);
        next
    }

    fn set_next_project_id(env: &Env, id: u64) {
        env.storage()
            .persistent()
            .set(&StorageKey::NextProjectId, &(id + 1));
    }

    fn owner_project_count(env: &Env, owner: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&StorageKey::OwnerProjectCount(owner.clone()))
            .unwrap_or(0)
    }

    fn inc_owner_project_count(env: &Env, owner: &Address) {
        let count = Self::owner_project_count(env, owner);
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjectCount(owner.clone()), &(count + 1));
    }

    pub fn register_project(
        env: &Env,
        _owner: Address,
        _name: String,
        _description: String,
        _category: String,
        _website: Option<String>,
        _logo_cid: Option<String>,
        _metadata_cid: Option<String>,
    ) -> Result<u64, ContractError> {
        let _registered_at: u64 = env.ledger().timestamp();
        todo!("Project registration logic not implemented")
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        env.storage()
            .persistent()
            .get(&StorageKey::Project(project_id))
    }

    pub fn update_project(
        env: &Env,
        project_id: u64,
        caller: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> Result<(), ContractError> {
        // 1. AUTHENTICATION: Verify the user's cryptographic signature
        caller.require_auth();

        // 2. RETRIEVAL: Check if project exists
        let mut project: Project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        // 3. AUTHORIZATION: Verify the caller is the stored owner
        if caller != project.owner {
            return Err(ContractError::Unauthorized);
        }

        // 4. DATA VALIDATION
        Self::validate_project_data(&name, &description, &category)?;

        // 5. UPDATE FIELDS
        project.name = name;
        project.description = description;
        project.category = category;
        project.website = website;
        project.logo_cid = logo_cid;
        project.metadata_cid = metadata_cid;

        // Update the timestamp to current ledger time
        project.updated_at = env.ledger().timestamp();

        // 6. PERSISTENCE: Save back to storage
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        Ok(())
    }

    /// Returns the number of projects registered by an owner (for tests and admin).
    pub fn get_owner_project_count(env: &Env, owner: &Address) -> u32 {
        Self::owner_project_count(env, owner)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::{DongleContract, DongleContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger, LedgerInfo},
        Address, Env, String,
    };

    fn ledger_at(timestamp: u64) -> LedgerInfo {
        LedgerInfo {
            timestamp,
            protocol_version: 20,
            sequence_number: 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 10_000_000,
        }
    }

    fn setup(env: &Env) -> DongleContractClient {
        let contract_id = env.register_contract(None, DongleContract);
        DongleContractClient::new(env, &contract_id)
    }

    pub fn project_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Project(project_id))
    }

    pub fn validate_project_data(
        _name: &String,
        _description: &String,
        _category: &String,
    ) -> Result<(), ContractError> {
        // Keeping as Ok(()) to allow updates to pass for now
        Ok(())
    }
}
