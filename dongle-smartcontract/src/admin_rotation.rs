//! Automatic admin-key rotation schedules — issue #900.
//!
//! Long-lived admin keys are a security risk: a leaked key stays valid until
//! somebody notices. This module lets the contract *enforce* periodic
//! rotation of the admin set:
//!
//! 1. An admin installs a **rotation schedule** with a fixed interval
//!    (30, 60 or 90 days) and a **replacement pool** of addresses that may
//!    be promoted into the admin set.
//! 2. Before the rotation runs, the operator works through a **pre-flight
//!    checklist** (schedule configured, replacement pool healthy, more than
//!    one admin, rotation actually due, …) and confirms it.
//! 3. [`AdminRotation::execute_scheduled_rotation`] — callable by any
//!    authenticated keeper — rotates the **oldest** admin key out of the set
//!    and promotes the next replacement candidate, emitting **notifications**
//!    for every affected key holder.
//!
//! The rotation is deliberately conservative:
//!
//! * it never removes the last admin (the contract would be bricked),
//! * it never runs before the schedule is due,
//! * it never runs before an admin confirmed the pre-flight checklist,
//! * it never promotes an address that is already an admin.
//!
//! See `docs/ADMIN_ROTATION_SCHEDULES.md` for the operator guide.

use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::{MAX_ROTATION_POOL_SIZE, SECONDS_PER_DAY};
use crate::errors::ContractError;
use crate::events::{
    publish_admin_added_event, publish_admin_removed_event, publish_admin_rotation_executed_event,
    publish_admin_rotation_notification_event, publish_admin_rotation_preflight_event,
    publish_admin_rotation_scheduled_event,
};
use crate::pagination::paginate;
use crate::storage_keys::{AdminRotationKey, StorageKey};
use crate::types::{
    AdminRotationNotification, AdminRotationRecord, AdminRotationSchedule, RotationChecklistItem,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct AdminRotation;

impl AdminRotation {
    // ── Schedule ─────────────────────────────────────────────────────────

    /// Install (or replace) the rotation schedule (admin-only).
    ///
    /// `interval_days` must be one of the supported intervals (30, 60 or 90).
    /// The next rotation becomes due `interval_days` after this call.
    pub fn set_rotation_schedule(
        env: &Env,
        admin: Address,
        interval_days: u32,
        replacement_pool: Vec<Address>,
    ) -> Result<AdminRotationSchedule, ContractError> {
        require_admin_auth(env, &admin)?;

        if !Self::is_supported_interval(interval_days) {
            return Err(ContractError::InvalidRotationInterval);
        }
        if replacement_pool.len() > MAX_ROTATION_POOL_SIZE {
            return Err(ContractError::InvalidProjectData);
        }

        // The pool must not contain duplicates or current admins.
        let mut seen: Vec<Address> = Vec::new(env);
        for candidate in replacement_pool.iter() {
            if seen.iter().any(|c| c == candidate) {
                return Err(ContractError::InvalidProjectData);
            }
            if AdminManager::is_admin(env, &candidate) {
                return Err(ContractError::InvalidProjectData);
            }
            seen.push_back(candidate);
        }

        let now = env.ledger().timestamp();
        let next_rotation_at = now.saturating_add(Self::interval_seconds(interval_days));

        // Backfill the join timestamp for admins that were added before the
        // schedule existed, so that installing a schedule never rotates an
        // admin out immediately.
        let admins = AdminManager::get_admin_list(env);
        for existing in admins.iter() {
            if Self::get_admin_joined_at(env, &existing).is_none() {
                env.storage()
                    .persistent()
                    .set(&AdminRotationKey::AdminJoinedAt(existing.clone()), &now);
            }
        }

        let schedule = AdminRotationSchedule {
            enabled: true,
            interval_days,
            next_rotation_at,
            last_rotation_at: None,
            replacement_pool: replacement_pool.clone(),
            preflight_confirmed: false,
            preflight_confirmed_at: None,
            rotation_count: 0,
        };

        env.storage()
            .persistent()
            .set(&AdminRotationKey::Schedule, &schedule);

        publish_admin_rotation_scheduled_event(
            env,
            interval_days,
            next_rotation_at,
            admin,
            replacement_pool.len(),
        );

        Ok(schedule)
    }

    /// The active rotation schedule, if any.
    pub fn get_rotation_schedule(env: &Env) -> Option<AdminRotationSchedule> {
        env.storage().persistent().get(&AdminRotationKey::Schedule)
    }

    // ── Pre-flight checklist ─────────────────────────────────────────────

    /// Build the rotation pre-flight checklist.
    ///
    /// The checklist is a machine-readable list of checks so the operator UI
    /// (or a CLI) can render it and the operator can confirm it on-chain.
    pub fn get_rotation_preflight_checklist(
        env: &Env,
        admin: Address,
    ) -> Result<Vec<RotationChecklistItem>, ContractError> {
        admin.require_auth();

        let schedule = Self::get_rotation_schedule(env);
        let now = env.ledger().timestamp();

        let configured = schedule.is_some();
        let interval_supported = match &schedule {
            Some(s) => Self::is_supported_interval(s.interval_days),
            None => false,
        };
        let pool_not_empty = match &schedule {
            Some(s) => s.replacement_pool.len() > 0,
            None => false,
        };
        let pool_candidates_valid = match &schedule {
            Some(s) => {
                let mut distinct = true;
                let mut all_non_admin = true;
                let mut seen: Vec<Address> = Vec::new(env);
                for candidate in s.replacement_pool.iter() {
                    if seen.iter().any(|c| c == candidate) {
                        distinct = false;
                    }
                    if AdminManager::is_admin(env, &candidate) {
                        all_non_admin = false;
                    }
                    seen.push_back(candidate);
                }
                distinct && all_non_admin
            }
            None => false,
        };
        let admin_count_ok = AdminManager::get_admin_count(env) > 1;
        let due = match &schedule {
            Some(s) => s.enabled && now >= s.next_rotation_at,
            None => false,
        };

        let mut items: Vec<RotationChecklistItem> = Vec::new(env);
        items.push_back(RotationChecklistItem {
            code: String::from_str(env, "schedule_configured"),
            description: String::from_str(env, "A rotation schedule is configured"),
            satisfied: configured,
        });
        items.push_back(RotationChecklistItem {
            code: String::from_str(env, "interval_supported"),
            description: String::from_str(env, "Rotation interval is 30, 60 or 90 days"),
            satisfied: interval_supported,
        });
        items.push_back(RotationChecklistItem {
            code: String::from_str(env, "replacement_pool_not_empty"),
            description: String::from_str(env, "At least one replacement candidate is available"),
            satisfied: pool_not_empty,
        });
        items.push_back(RotationChecklistItem {
            code: String::from_str(env, "replacement_candidates_valid"),
            description: String::from_str(
                env,
                "Replacement candidates are distinct and not current admins",
            ),
            satisfied: pool_candidates_valid,
        });
        items.push_back(RotationChecklistItem {
            code: String::from_str(env, "more_than_one_admin"),
            description: String::from_str(env, "More than one admin is registered"),
            satisfied: admin_count_ok,
        });
        items.push_back(RotationChecklistItem {
            code: String::from_str(env, "rotation_due"),
            description: String::from_str(env, "The scheduled rotation is due"),
            satisfied: due,
        });

        Ok(items)
    }

    /// Confirm the pre-flight checklist (admin-only).
    ///
    /// Every check must pass before the rotation can be executed — this is the
    /// "confirmation flow" that keeps an unattended keeper from rotating keys
    /// at the wrong moment.
    pub fn confirm_rotation_preflight(
        env: &Env,
        admin: Address,
    ) -> Result<AdminRotationSchedule, ContractError> {
        require_admin_auth(env, &admin)?;

        let mut schedule =
            Self::get_rotation_schedule(env).ok_or(ContractError::RotationScheduleNotSet)?;

        let checklist = Self::get_rotation_preflight_checklist(env, admin.clone())?;
        let mut all_satisfied = true;
        for item in checklist.iter() {
            if !item.satisfied {
                all_satisfied = false;
            }
        }
        if !all_satisfied {
            return Err(ContractError::RotationPreflightChecklistIncomplete);
        }

        schedule.preflight_confirmed = true;
        schedule.preflight_confirmed_at = Some(env.ledger().timestamp());
        env.storage()
            .persistent()
            .set(&AdminRotationKey::Schedule, &schedule);

        let rotation_id = Self::next_rotation_id(env);
        publish_admin_rotation_preflight_event(
            env,
            rotation_id,
            admin,
            checklist.len(),
            all_satisfied,
        );

        Ok(schedule)
    }

    // ── Execution ────────────────────────────────────────────────────────

    /// Execute the scheduled rotation.
    ///
    /// Callable by any authenticated keeper once the rotation is due and an
    /// admin has confirmed the pre-flight checklist. The oldest admin key is
    /// removed and the next replacement candidate is promoted; both the
    /// incoming and the outgoing key holder receive a notification.
    pub fn execute_scheduled_rotation(
        env: &Env,
        caller: Address,
    ) -> Result<AdminRotationRecord, ContractError> {
        caller.require_auth();

        let mut schedule =
            Self::get_rotation_schedule(env).ok_or(ContractError::RotationScheduleNotSet)?;

        if !schedule.enabled {
            return Err(ContractError::RotationScheduleNotSet);
        }
        if env.ledger().timestamp() < schedule.next_rotation_at {
            return Err(ContractError::RotationNotDue);
        }
        if !schedule.preflight_confirmed {
            return Err(ContractError::RotationPreflightNotConfirmed);
        }

        let admins = AdminManager::get_admin_list(env);
        if admins.len() <= 1 {
            return Err(ContractError::RotationWouldRemoveLastAdmin);
        }

        // The admin that joined the longest ago is the one that rotates out.
        let mut oldest: Option<Address> = None;
        let mut oldest_joined: u64 = u64::MAX;
        for admin in admins.iter() {
            let joined = Self::get_admin_joined_at(env, &admin).unwrap_or(0);
            if joined < oldest_joined {
                oldest_joined = joined;
                oldest = Some(admin.clone());
            }
        }
        let outgoing = oldest.ok_or(ContractError::NoAdminsDueForRotation)?;

        // The next candidate of the pool that is not an admin yet is promoted.
        let mut incoming: Option<Address> = None;
        for candidate in schedule.replacement_pool.iter() {
            if !AdminManager::is_admin(env, &candidate) {
                incoming = Some(candidate.clone());
                break;
            }
        }

        let mut removed_admins: Vec<Address> = Vec::new(env);
        let mut added_admins: Vec<Address> = Vec::new(env);

        // Remove the outgoing admin.
        env.storage()
            .persistent()
            .remove(&StorageKey::Admin(outgoing.clone()));
        let remaining = Utils::remove_item_from_vec(env, &admins, &outgoing);
        env.storage()
            .persistent()
            .set(&StorageKey::AdminList, &remaining);
        env.storage()
            .persistent()
            .remove(&AdminRotationKey::AdminJoinedAt(outgoing.clone()));
        publish_admin_removed_event(env, outgoing.clone());
        removed_admins.push_back(outgoing.clone());

        // Promote the replacement admin.
        if let Some(candidate) = incoming {
            env.storage()
                .persistent()
                .set(&StorageKey::Admin(candidate.clone()), &true);
            let mut new_admins = remaining;
            new_admins.push_back(candidate.clone());
            env.storage()
                .persistent()
                .set(&StorageKey::AdminList, &new_admins);
            Self::record_admin_join(env, &candidate);
            publish_admin_added_event(env, candidate.clone());
            added_admins.push_back(candidate.clone());
        }

        let rotation_id = Self::next_rotation_id(env);
        let now = env.ledger().timestamp();

        // Notifications for every affected key holder.
        for admin in removed_admins.iter() {
            Self::record_notification(env, rotation_id, admin.clone(), true);
            publish_admin_rotation_notification_event(env, rotation_id, admin.clone(), true);
        }
        for admin in added_admins.iter() {
            Self::record_notification(env, rotation_id, admin.clone(), false);
            publish_admin_rotation_notification_event(env, rotation_id, admin.clone(), false);
        }

        let record = AdminRotationRecord {
            rotation_id,
            removed_admins: removed_admins.clone(),
            added_admins: added_admins.clone(),
            executed_at: now,
            executed_by: caller.clone(),
        };
        env.storage()
            .persistent()
            .set(&AdminRotationKey::Record(rotation_id), &record);

        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AdminRotationKey::RecordIds)
            .unwrap_or_else(|| Vec::new(env));
        ids.push_back(rotation_id);
        env.storage()
            .persistent()
            .set(&AdminRotationKey::RecordIds, &ids);

        // Re-arm the schedule and require a fresh pre-flight confirmation.
        schedule.last_rotation_at = Some(now);
        schedule.next_rotation_at =
            now.saturating_add(Self::interval_seconds(schedule.interval_days));
        schedule.rotation_count = schedule.rotation_count.saturating_add(1);
        schedule.preflight_confirmed = false;
        schedule.preflight_confirmed_at = None;
        env.storage()
            .persistent()
            .set(&AdminRotationKey::Schedule, &schedule);

        publish_admin_rotation_executed_event(
            env,
            rotation_id,
            caller,
            removed_admins,
            added_admins,
            schedule.next_rotation_at,
        );

        Ok(record)
    }

    // ── Reads ────────────────────────────────────────────────────────────

    /// A single rotation record.
    pub fn get_admin_rotation_record(env: &Env, rotation_id: u64) -> Option<AdminRotationRecord> {
        env.storage()
            .persistent()
            .get(&AdminRotationKey::Record(rotation_id))
    }

    /// All rotation records with pagination (oldest first).
    pub fn list_admin_rotation_records(
        env: &Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<AdminRotationRecord> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AdminRotationKey::RecordIds)
            .unwrap_or_else(|| Vec::new(env));
        let page = paginate(env, &ids, start_index, limit);
        let mut out = Vec::new(env);
        for id in page.iter() {
            if let Some(record) = Self::get_admin_rotation_record(env, id) {
                out.push_back(record);
            }
        }
        out
    }

    /// The notifications emitted by a rotation (off-chain operators use them
    /// to notify the affected key holders).
    pub fn get_admin_rotation_notifications(
        env: &Env,
        rotation_id: u64,
    ) -> Vec<AdminRotationNotification> {
        let count: u32 = env
            .storage()
            .persistent()
            .get(&AdminRotationKey::NotificationCount(rotation_id))
            .unwrap_or(0);
        let mut out = Vec::new(env);
        let mut index: u32 = 0;
        while index < count {
            if let Some(notification) = env
                .storage()
                .persistent()
                .get(&AdminRotationKey::Notification(rotation_id, index))
            {
                out.push_back(notification);
            }
            index += 1;
        }
        out
    }

    // ── Bookkeeping ──────────────────────────────────────────────────────

    /// Record the moment an address joined the admin set.
    ///
    /// Called by the admin manager whenever an admin is added so that the
    /// rotation knows which key is the oldest.
    pub fn record_admin_join(env: &Env, admin: &Address) {
        let now = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&AdminRotationKey::AdminJoinedAt(admin.clone()), &now);
    }

    /// When the given address joined the admin set.
    pub fn get_admin_joined_at(env: &Env, admin: &Address) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&AdminRotationKey::AdminJoinedAt(admin.clone()))
    }

    /// Whether the given interval is one of the supported rotation intervals.
    pub fn is_supported_interval(interval_days: u32) -> bool {
        interval_days == 30 || interval_days == 60 || interval_days == 90
    }

    fn interval_seconds(interval_days: u32) -> u64 {
        (interval_days as u64).saturating_mul(SECONDS_PER_DAY)
    }

    fn next_rotation_id(env: &Env) -> u64 {
        let next: u64 = env
            .storage()
            .persistent()
            .get(&AdminRotationKey::NextRotationId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&AdminRotationKey::NextRotationId, &(next + 1));
        next
    }

    fn record_notification(env: &Env, rotation_id: u64, admin: Address, removed: bool) {
        let count: u32 = env
            .storage()
            .persistent()
            .get(&AdminRotationKey::NotificationCount(rotation_id))
            .unwrap_or(0);
        let notification = AdminRotationNotification {
            rotation_id,
            admin,
            removed,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(
            &AdminRotationKey::Notification(rotation_id, count),
            &notification,
        );
        env.storage().persistent().set(
            &AdminRotationKey::NotificationCount(rotation_id),
            &(count + 1),
        );
    }
}
