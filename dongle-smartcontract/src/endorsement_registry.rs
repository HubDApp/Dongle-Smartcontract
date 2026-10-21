use crate::constants::{MAX_ENDORSEMENT_AREAS, MAX_ENDORSEMENT_AREA_LEN, MAX_PAGE_LIMIT};
use crate::errors::ContractError;
use crate::events::{
    publish_endorsement_renewed_event, publish_project_endorsed_event,
    publish_project_unendorsed_event,
};
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{EndorsementRecordKey, ExtensionKey};
use crate::storage_manager::StorageManager;
use crate::types::Endorsement;
use soroban_sdk::{Address, Env, String, Vec};

pub struct EndorsementRegistry;

impl EndorsementRegistry {
    pub fn endorse_project(env: &Env, project_id: u64, user: Address) -> Result<(), ContractError> {
        Self::endorse_project_with_conditions(env, project_id, user, Vec::new(env), None)
            .map(|_| ())
    }

    /// Endorse a project for specific expertise `areas` (empty = general
    /// endorsement), optionally expiring at `expires_at`.  Returns the
    /// endorsement ID.
    pub fn endorse_project_with_conditions(
        env: &Env,
        project_id: u64,
        user: Address,
        areas: Vec<String>,
        expires_at: Option<u64>,
    ) -> Result<u64, ContractError> {
        user.require_auth();
        Self::endorse_internal(env, project_id, user, areas, expires_at, None)
    }

    /// Endorsement logic shared by direct and delegated (#919) endorsements.
    /// The caller is responsible for authenticating `user`.
    pub(crate) fn endorse_internal(
        env: &Env,
        project_id: u64,
        user: Address,
        areas: Vec<String>,
        expires_at: Option<u64>,
        on_behalf_of: Option<Address>,
    ) -> Result<u64, ContractError> {
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        Self::validate_areas(&areas)?;
        Self::validate_expiry(env, expires_at)?;

        Self::ensure_indexed(env, project_id);
        if Self::has_endorsed(env, project_id, &user) {
            return Err(ContractError::AlreadyEndorsed);
        }

        let count = Self::current_count(env, project_id);
        env.storage()
            .persistent()
            .set(&ExtensionKey::EndorsementAt(project_id, count), &user);
        env.storage().persistent().set(
            &ExtensionKey::EndorsementIndex(project_id, user.clone()),
            &count,
        );
        let new_count = count + 1;
        env.storage()
            .persistent()
            .set(&ExtensionKey::EndorsementCount(project_id), &new_count);
        StorageManager::extend_endorsement_entry_ttl(env, project_id, &user, count);

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_endorsements_ttl(env, project_id);

        let endorsement_id =
            Self::create_record(env, project_id, &user, areas, expires_at, on_behalf_of);

        publish_project_endorsed_event(env, project_id, user);
        Ok(endorsement_id)
    }

    pub fn unendorse_project(
        env: &Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        user.require_auth();

        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        Self::ensure_indexed(env, project_id);
        if !Self::has_endorsed(env, project_id, &user) {
            return Err(ContractError::NotEndorsed);
        }

        let index: u32 = env
            .storage()
            .persistent()
            .get(&ExtensionKey::EndorsementIndex(project_id, user.clone()))
            .ok_or(ContractError::NotEndorsed)?;
        let count = Self::current_count(env, project_id);
        let last_index = count - 1;
        if index != last_index {
            let last_user: Address = env
                .storage()
                .persistent()
                .get(&ExtensionKey::EndorsementAt(project_id, last_index))
                .ok_or(ContractError::NotEndorsed)?;
            env.storage().persistent().set(
                &ExtensionKey::EndorsementAt(project_id, index),
                &last_user,
            );
            env.storage()
                .persistent()
                .set(&ExtensionKey::EndorsementAt(project_id, index), &last_user);
            env.storage().persistent().set(
                &ExtensionKey::EndorsementIndex(project_id, last_user.clone()),
                &index,
            );
            StorageManager::extend_endorsement_entry_ttl(env, project_id, &last_user, index);
        }
        env.storage()
            .persistent()
            .remove(&ExtensionKey::EndorsementAt(project_id, last_index));
        env.storage()
            .persistent()
            .remove(&ExtensionKey::EndorsementIndex(project_id, user.clone()));
        let new_count = last_index;
        env.storage()
            .persistent()
            .set(&ExtensionKey::EndorsementCount(project_id), &new_count);

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_endorsements_ttl(env, project_id);

        Self::remove_record(env, project_id, &user);

        publish_project_unendorsed_event(env, project_id, user);
        Ok(())
    }

    // ── Conditional endorsements (#918) ───────────────────────────────────

    /// Renew a time-limited endorsement, extending it from now by its
    /// original validity period.  Works on already-expired endorsements.
    /// Returns the new expiry timestamp.
    pub fn renew_endorsement(
        env: &Env,
        endorser: Address,
        endorsement_id: u64,
    ) -> Result<u64, ContractError> {
        endorser.require_auth();

        let mut record =
            Self::get_endorsement(env, endorsement_id).ok_or(ContractError::EndorsementNotFound)?;
        if record.endorser != endorser {
            return Err(ContractError::Unauthorized);
        }
        let expires_at = record.expires_at.ok_or(ContractError::EndorsementNotRenewable)?;

        let started_at = record.renewed_at.unwrap_or(record.created_at);
        let period = expires_at.saturating_sub(started_at).max(1);
        let now = env.ledger().timestamp();
        let new_expires_at = now.saturating_add(period);

        record.renewed_at = Some(now);
        record.expires_at = Some(new_expires_at);
        env.storage()
            .persistent()
            .set(&EndorsementRecordKey::Record(endorsement_id), &record);

        publish_endorsement_renewed_event(
            env,
            endorsement_id,
            record.project_id,
            endorser,
            new_expires_at,
        );
        Ok(new_expires_at)
    }

    pub fn get_endorsement(env: &Env, endorsement_id: u64) -> Option<Endorsement> {
        env.storage()
            .persistent()
            .get(&EndorsementRecordKey::Record(endorsement_id))
    }

    /// The endorsement record `user` holds on `project_id`, if any.
    pub fn get_user_project_endorsement(
        env: &Env,
        project_id: u64,
        user: &Address,
    ) -> Option<Endorsement> {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&EndorsementRecordKey::ProjectEndorser(project_id, user.clone()))?;
        Self::get_endorsement(env, id)
    }

    /// Whether `record` is still in force at the current ledger time.
    pub fn is_endorsement_active(env: &Env, record: &Endorsement) -> bool {
        match record.expires_at {
            Some(expires_at) => env.ledger().timestamp() < expires_at,
            None => true,
        }
    }

    /// Whether `user` holds an unexpired endorsement on `project_id`.
    /// Legacy endorsements without a record never expire.
    pub fn has_active_endorsement(env: &Env, project_id: u64, user: &Address) -> bool {
        if !Self::has_endorsed(env, project_id, user) {
            return false;
        }
        match Self::get_user_project_endorsement(env, project_id, user) {
            Some(record) => Self::is_endorsement_active(env, &record),
            None => true,
        }
    }

    /// Active (unexpired) endorsements of `project_id` covering `area`,
    /// paginated over the active results.
    pub fn get_endorsements_by_area(
        env: &Env,
        project_id: u64,
        area: String,
        start_index: u32,
        limit: u32,
    ) -> Vec<Endorsement> {
        let effective_limit = if limit == 0 {
            MAX_PAGE_LIMIT
        } else {
            limit.min(MAX_PAGE_LIMIT)
        };
        let ids = Self::area_ids(env, project_id, &area);

        let mut result = Vec::new(env);
        let mut active_seen = 0u32;
        for id in ids.iter() {
            if result.len() >= effective_limit {
                break;
            }
            if let Some(record) = Self::get_endorsement(env, id) {
                if !Self::is_endorsement_active(env, &record) {
                    continue;
                }
                if active_seen >= start_index {
                    result.push_back(record);
                }
                active_seen += 1;
            }
        }
        result
    }

    /// Filter `project_id`'s endorsers down to those whose active
    /// endorsement covers every area in `areas`.
    pub fn filter_endorsers_by_areas(
        env: &Env,
        project_id: u64,
        areas: Vec<String>,
    ) -> Vec<Address> {
        let mut result = Vec::new(env);
        let Some(first) = areas.get(0) else {
            return result;
        };
        for id in Self::area_ids(env, project_id, &first).iter() {
            if let Some(record) = Self::get_endorsement(env, id) {
                if Self::is_endorsement_active(env, &record)
                    && areas.iter().all(|area| record.areas.contains(&area))
                {
                    result.push_back(record.endorser);
                }
            }
        }
        result
    }

    pub(crate) fn validate_areas(areas: &Vec<String>) -> Result<(), ContractError> {
        if areas.len() > MAX_ENDORSEMENT_AREAS {
            return Err(ContractError::TooManyEndorsementAreas);
        }
        for i in 0..areas.len() {
            let area = areas.get(i).ok_or(ContractError::InvalidEndorsementArea)?;
            if area.len() == 0 || area.len() > MAX_ENDORSEMENT_AREA_LEN {
                return Err(ContractError::InvalidEndorsementArea);
            }
            for j in 0..i {
                if areas.get(j) == Some(area.clone()) {
                    return Err(ContractError::InvalidEndorsementArea);
                }
            }
        }
        Ok(())
    }

    fn validate_expiry(env: &Env, expires_at: Option<u64>) -> Result<(), ContractError> {
        if let Some(expires_at) = expires_at {
            if expires_at <= env.ledger().timestamp() {
                return Err(ContractError::InvalidEndorsementExpiry);
            }
        }
        Ok(())
    }

    fn area_ids(env: &Env, project_id: u64, area: &String) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&EndorsementRecordKey::AreaIndex(project_id, area.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn create_record(
        env: &Env,
        project_id: u64,
        user: &Address,
        areas: Vec<String>,
        expires_at: Option<u64>,
        on_behalf_of: Option<Address>,
    ) -> u64 {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&EndorsementRecordKey::NextId)
            .unwrap_or(1u64);
        env.storage()
            .persistent()
            .set(&EndorsementRecordKey::NextId, &(id + 1));

        for area in areas.iter() {
            let mut ids = Self::area_ids(env, project_id, &area);
            ids.push_back(id);
            env.storage()
                .persistent()
                .set(&EndorsementRecordKey::AreaIndex(project_id, area), &ids);
        }

        let record = Endorsement {
            id,
            project_id,
            endorser: user.clone(),
            areas,
            created_at: env.ledger().timestamp(),
            renewed_at: None,
            expires_at,
            delegated_to: None,
            on_behalf_of,
        };
        env.storage()
            .persistent()
            .set(&EndorsementRecordKey::Record(id), &record);
        env.storage().persistent().set(
            &EndorsementRecordKey::ProjectEndorser(project_id, user.clone()),
            &id,
        );
        id
    }

    fn remove_record(env: &Env, project_id: u64, user: &Address) {
        let Some(record) = Self::get_user_project_endorsement(env, project_id, user) else {
            return;
        };
        for area in record.areas.iter() {
            let ids = Self::area_ids(env, project_id, &area);
            let mut remaining = Vec::new(env);
            for id in ids.iter() {
                if id != record.id {
                    remaining.push_back(id);
                }
            }
            env.storage()
                .persistent()
                .set(&EndorsementRecordKey::AreaIndex(project_id, area), &remaining);
        }
        env.storage()
            .persistent()
            .remove(&EndorsementRecordKey::Record(record.id));
        env.storage()
            .persistent()
            .remove(&EndorsementRecordKey::ProjectEndorser(project_id, user.clone()));
    }

    pub fn get_endorsement_count(env: &Env, project_id: u64) -> u32 {
        Self::current_count(env, project_id)
    }

    pub fn get_project_endorsements(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<Address> {
        let count = Self::current_count(env, project_id);
        let effective_limit = if limit == 0 {
            MAX_PAGE_LIMIT
        } else {
            limit.min(MAX_PAGE_LIMIT)
        };
        let mut result = Vec::new(env);
        let end = start_index.saturating_add(effective_limit).min(count);
        let mut index = start_index;
        while index < end {
            if let Some(user) = env
                .storage()
                .persistent()
                .get(&ExtensionKey::EndorsementAt(project_id, index))
            {
                result.push_back(user);
            }
            index += 1;
        }
        if result.is_empty() && count > 0 && start_index < count {
            let legacy: Vec<Address> = env
                .storage()
                .persistent()
                .get(&ExtensionKey::ProjectEndorsements(project_id))
                .unwrap_or_else(|| Vec::new(env));
            return paginate(env, &legacy, start_index, effective_limit);
        }
        result
    }

    pub fn has_endorsed(env: &Env, project_id: u64, user: &Address) -> bool {
        env.storage()
            .persistent()
            .has(&ExtensionKey::EndorsementIndex(project_id, user.clone()))
            || env
                .storage()
                .persistent()
                .get::<_, Vec<Address>>(&ExtensionKey::ProjectEndorsements(project_id))
                .map(|endorsements| endorsements.contains(user))
                .unwrap_or(false)
    }

    fn current_count(env: &Env, project_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get(&ExtensionKey::EndorsementCount(project_id))
            .or_else(|| {
                env.storage()
                    .persistent()
                    .get::<_, Vec<Address>>(&ExtensionKey::ProjectEndorsements(project_id))
                    .map(|endorsements| endorsements.len())
            })
            .unwrap_or(0)
    }

    fn ensure_indexed(env: &Env, project_id: u64) {
        if Self::current_count(env, project_id) == 0
            || env
                .storage()
                .persistent()
                .has(&ExtensionKey::EndorsementAt(project_id, 0))
        {
            return;
        }

        let legacy: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectEndorsements(project_id))
            .unwrap_or_else(|| Vec::new(env));
        for index in 0..legacy.len() {
            if let Some(user) = legacy.get(index) {
                env.storage().persistent().set(
                    &ExtensionKey::EndorsementAt(project_id, index),
                    &user,
                );
                env.storage().persistent().set(
                    &ExtensionKey::EndorsementIndex(project_id, user),
                    &index,
                );
            }
            let user = legacy.get(index).expect("legacy endorsement index");
            env.storage()
                .persistent()
                .set(&ExtensionKey::EndorsementAt(project_id, index), &user);
            env.storage()
                .persistent()
                .set(&ExtensionKey::EndorsementIndex(project_id, user), &index);
        }
    }
}
