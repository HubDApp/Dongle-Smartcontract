//! Scheduled auto-rotation of featured projects (issue #927).
//!
//! An admin schedules an ordered list of projects and a rotation period in
//! days. Slot `n` runs from `started_at + n * period` to
//! `started_at + (n + 1) * period`, featuring
//! `projects[n % projects.len()]`, so the rotation wraps around indefinitely.
//!
//! ## Lazy evaluation
//! Nothing has to run on chain for the rotation to advance: every read
//! derives the current slot from the ledger timestamp. The stored schedule is
//! only rewritten by `advance_featured_rotation`, which anyone may call to
//! persist the current slot and emit a `FEATURED/ROTATED` event for indexers.

use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::constants::{MAX_FEATURED_PROJECTS, MAX_ROTATION_PERIOD_DAYS, ROTATION_SECONDS_PER_DAY};
use crate::errors::ContractError;
use crate::events::{
    publish_featured_rotation_advanced_event, publish_featured_rotation_scheduled_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::FeaturedRotationKey;
use crate::types::{AdminActionType, FeaturedRotationSchedule};
use soroban_sdk::{Address, Env, Vec};

pub struct FeaturedRotationRegistry;

impl FeaturedRotationRegistry {
    /// Schedule (or replace) the featured rotation. Admin-only.
    ///
    /// `projects` must hold 1..=`MAX_FEATURED_PROJECTS` distinct, existing,
    /// non-archived projects. `rotation_period_days` must be in
    /// 1..=`MAX_ROTATION_PERIOD_DAYS`. The first project is featured
    /// immediately.
    pub fn schedule_featured_rotation(
        env: &Env,
        admin: Address,
        projects: Vec<u64>,
        rotation_period_days: u32,
    ) -> Result<FeaturedRotationSchedule, ContractError> {
        require_admin_auth(env, &admin)?;

        if projects.is_empty() || projects.len() > MAX_FEATURED_PROJECTS {
            return Err(ContractError::InvalidInput);
        }
        if rotation_period_days == 0 || rotation_period_days > MAX_ROTATION_PERIOD_DAYS {
            return Err(ContractError::InvalidInput);
        }
        let mut distinct: Vec<u64> = Vec::new(env);
        for project_id in projects.iter() {
            if distinct.contains(project_id) {
                return Err(ContractError::InvalidInput);
            }
            let project =
                ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
            if project.archived {
                return Err(ContractError::InvalidInput);
            }
            distinct.push_back(project_id);
        }

        let now = env.ledger().timestamp();
        let mut schedule = FeaturedRotationSchedule {
            projects,
            rotation_period_days,
            started_at: now,
            current_index: 0,
            current_project_id: 0,
            next_rotation_at: 0,
            rotations_completed: 0,
            scheduled_by: admin.clone(),
        };
        Self::evaluate(now, &mut schedule);
        env.storage()
            .persistent()
            .set(&FeaturedRotationKey::Schedule, &schedule);

        publish_featured_rotation_scheduled_event(env, &schedule);
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::FeaturedRotationScheduled,
            Some(schedule.current_project_id),
            None,
            None,
        );

        Ok(schedule)
    }

    /// The rotation schedule evaluated at the current ledger timestamp, or
    /// `None` when no rotation is scheduled. Read-only.
    pub fn get_featured_rotation_schedule(env: &Env) -> Option<FeaturedRotationSchedule> {
        let mut schedule: FeaturedRotationSchedule = env
            .storage()
            .persistent()
            .get(&FeaturedRotationKey::Schedule)?;
        Self::evaluate(env.ledger().timestamp(), &mut schedule);
        Some(schedule)
    }

    /// The currently featured project of the rotation, if one is scheduled.
    pub fn get_current_rotation_project(env: &Env) -> Option<u64> {
        Self::get_featured_rotation_schedule(env).map(|s| s.current_project_id)
    }

    /// Persist the current slot and emit `FEATURED/ROTATED` if the featured
    /// project changed since the last persisted state. Callable by anyone;
    /// the result is fully determined by the ledger timestamp.
    pub fn advance_featured_rotation(env: &Env) -> Result<FeaturedRotationSchedule, ContractError> {
        let stored: FeaturedRotationSchedule = env
            .storage()
            .persistent()
            .get(&FeaturedRotationKey::Schedule)
            .ok_or(ContractError::InvalidStatus)?;
        let mut schedule = stored.clone();
        Self::evaluate(env.ledger().timestamp(), &mut schedule);

        if schedule.rotations_completed != stored.rotations_completed {
            env.storage()
                .persistent()
                .set(&FeaturedRotationKey::Schedule, &schedule);
            if schedule.current_project_id != stored.current_project_id {
                publish_featured_rotation_advanced_event(env, stored.current_project_id, &schedule);
            }
        }
        Ok(schedule)
    }

    /// Remove the rotation schedule. Admin-only.
    pub fn clear_featured_rotation(env: &Env, admin: Address) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        env.storage()
            .persistent()
            .remove(&FeaturedRotationKey::Schedule);
        Ok(())
    }

    /// Derive the current slot of `schedule` at timestamp `now`.
    fn evaluate(now: u64, schedule: &mut FeaturedRotationSchedule) {
        let period = (schedule.rotation_period_days as u64).saturating_mul(ROTATION_SECONDS_PER_DAY);
        let elapsed = now.saturating_sub(schedule.started_at);
        let rotations = elapsed / period;
        let index = (rotations % schedule.projects.len() as u64) as u32;

        schedule.rotations_completed = rotations;
        schedule.current_index = index;
        schedule.current_project_id = schedule.projects.get(index).unwrap_or(0);
        schedule.next_rotation_at = schedule
            .started_at
            .saturating_add(rotations.saturating_add(1).saturating_mul(period));
    }
}
