use crate::events::{publish_project_endorsed_event, publish_project_unendorsed_event};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use soroban_sdk::{contracterror, Address, Env, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EndorsementError {
    AlreadyEndorsed = 1,
    NotEndorsed = 2,
}

pub struct EndorsementRegistry;

impl EndorsementRegistry {
    pub fn endorse_project(
        env: &Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), EndorsementError> {
        user.require_auth();

        if ProjectRegistry::get_project(env, project_id).is_none() {
            panic!("project not found");
        }

        if Self::has_endorsed(env, project_id, &user) {
            return Err(EndorsementError::AlreadyEndorsed);
        }

        let mut endorsements: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectEndorsements(project_id))
            .unwrap_or_else(|| Vec::new(env));
        endorsements.push_back(user.clone());
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
    ) -> Result<(), EndorsementError> {
        user.require_auth();

        if !Self::has_endorsed(env, project_id, &user) {
            return Err(EndorsementError::NotEndorsed);
        }

        let endorsements: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectEndorsements(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let mut new_endorsements: Vec<Address> = Vec::new(env);
        for i in 0..endorsements.len() {
            if let Some(e) = endorsements.get(i) {
                if e != user {
                    new_endorsements.push_back(e);
                }
            }
        }
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
