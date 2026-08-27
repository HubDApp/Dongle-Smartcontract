use crate::errors::ContractError;
use crate::events::{publish_project_endorsed_event, publish_project_unendorsed_event};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Vec};

pub struct EndorsementRegistry;

impl EndorsementRegistry {
    pub fn endorse_project(env: &Env, project_id: u64, user: Address) -> Result<(), ContractError> {
        user.require_auth();

        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if Self::has_endorsed(env, project_id, &user) {
            return Err(ContractError::AlreadyEndorsed);
        }

        let mut endorsements: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectEndorsements(project_id))
            .unwrap_or_else(|| Vec::new(env));
        let _ = Utils::add_unique_to_vec(&mut endorsements, &user);
        env.storage().persistent().set(
            &ExtensionKey::ProjectEndorsements(project_id),
            &endorsements,
        );

        let count: u32 = endorsements.len();
        env.storage()
            .persistent()
            .set(&ExtensionKey::EndorsementCount(project_id), &count);

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_endorsements_ttl(env, project_id);

        publish_project_endorsed_event(env, project_id, user);
        Ok(())
    }

    pub fn unendorse_project(
        env: &Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        user.require_auth();

        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if !Self::has_endorsed(env, project_id, &user) {
            return Err(ContractError::NotEndorsed);
        }

        let endorsements: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectEndorsements(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let new_endorsements = Utils::remove_item_from_vec(env, &endorsements, &user);
        env.storage().persistent().set(
            &ExtensionKey::ProjectEndorsements(project_id),
            &new_endorsements,
        );

        let count: u32 = new_endorsements.len();
        env.storage()
            .persistent()
            .set(&ExtensionKey::EndorsementCount(project_id), &count);

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_endorsements_ttl(env, project_id);

        publish_project_unendorsed_event(env, project_id, user);
        Ok(())
    }

    pub fn get_endorsement_count(env: &Env, project_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get(&ExtensionKey::EndorsementCount(project_id))
            .unwrap_or(0)
    }

    pub fn has_endorsed(env: &Env, project_id: u64, user: &Address) -> bool {
        let endorsements: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectEndorsements(project_id))
            .unwrap_or_else(|| Vec::new(env));
        endorsements.contains(user)
    }
}
