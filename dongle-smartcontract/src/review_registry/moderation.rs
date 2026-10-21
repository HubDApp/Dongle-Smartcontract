//! Batch review moderation: hide, delete, flag, or restore many reviews in one
//! call, with per-item results, an audit-log entry per action, and a per-admin
//! rate limit.

use crate::admin_action_log::AdminActionLog;
use crate::constants::{
    LEDGER_BUMP_CRITICAL, LEDGER_THRESHOLD_CRITICAL, MAX_MODERATION_BATCH_SIZE,
    MODERATION_ACTIONS_PER_WINDOW, MODERATION_RATE_LIMIT_WINDOW_SECONDS,
};
use crate::errors::ContractError;
use crate::events::{publish_review_flagged_event, publish_reviews_moderated_event};
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::{ReviewModerationKey, StorageKey};
use crate::types::{
    AdminActionType, ModerateAction, ModerateFailure, ModerateResult, Review, ReviewFlag,
};
use soroban_sdk::{contracttype, Address, Env, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ModerationWindow {
    bucket: u64,
    count: u32,
}

impl ReviewRegistry {
    /// Apply `action` to every `(project_id, reviewer)` in `reviews`.
    ///
    /// The batch as a whole fails only on auth, admin, size, or rate-limit
    /// errors — all checked before any write. Individual reviews that cannot
    /// be moderated (missing, already hidden, …) are reported in
    /// `ModerateResult::failed` without rolling back the rest.
    pub fn moderate_reviews_batch(
        env: &Env,
        admin: Address,
        reviews: Vec<(u64, Address)>,
        action: ModerateAction,
    ) -> Result<ModerateResult, ContractError> {
        admin.require_auth();

        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        if reviews.is_empty() || reviews.len() > MAX_MODERATION_BATCH_SIZE {
            return Err(ContractError::InvalidInput);
        }

        Self::consume_moderation_budget(env, &admin, reviews.len())?;

        let mut successful = Vec::new(env);
        let mut failed = Vec::new(env);
        for (project_id, reviewer) in reviews.iter() {
            match Self::apply_moderation(env, project_id, reviewer.clone(), &admin, action) {
                Ok(()) => successful.push_back((project_id, reviewer)),
                Err(err) => failed.push_back(ModerateFailure {
                    project_id,
                    reviewer,
                    error_code: err as u32,
                }),
            }
        }

        publish_reviews_moderated_event(env, admin, action, successful.len(), failed.len());

        Ok(ModerateResult {
            action,
            successful,
            failed,
        })
    }

    pub fn get_review_flag(env: &Env, project_id: u64, reviewer: Address) -> Option<ReviewFlag> {
        env.storage()
            .persistent()
            .get(&ReviewModerationKey::Flag(project_id, reviewer))
    }

    /// Moderation actions `admin` may still attempt in the current window.
    pub fn get_moderation_budget(env: &Env, admin: Address) -> u32 {
        MODERATION_ACTIONS_PER_WINDOW.saturating_sub(Self::moderation_used(env, &admin))
    }

    fn apply_moderation(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        admin: &Address,
        action: ModerateAction,
    ) -> Result<(), ContractError> {
        match action {
            ModerateAction::Hide => Self::apply_hide_review(env, project_id, reviewer, admin.clone()),
            ModerateAction::Delete => {
                Self::apply_admin_delete_review(env, project_id, reviewer.clone(), admin.clone())?;
                Self::clear_flag(env, project_id, &reviewer);
                Ok(())
            }
            ModerateAction::Flag => Self::apply_flag_review(env, project_id, reviewer, admin),
            ModerateAction::Restore => {
                Self::apply_restore_or_unflag(env, project_id, reviewer, admin)
            }
        }
    }

    fn apply_flag_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        admin: &Address,
    ) -> Result<(), ContractError> {
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }
        if !env
            .storage()
            .persistent()
            .has(&StorageKey::Review(project_id, reviewer.clone()))
        {
            return Err(ContractError::ReviewNotFound);
        }

        let flag_key = ReviewModerationKey::Flag(project_id, reviewer.clone());
        if env.storage().persistent().has(&flag_key) {
            return Err(ContractError::ReviewAlreadyFlagged);
        }

        env.storage().persistent().set(
            &flag_key,
            &ReviewFlag {
                flagged_by: admin.clone(),
                flagged_at: env.ledger().timestamp(),
            },
        );

        publish_review_flagged_event(env, project_id, reviewer.clone(), admin.clone());
        AdminActionLog::record_action(
            env,
            admin.clone(),
            AdminActionType::ReviewFlagged,
            Some(project_id),
            Some(reviewer),
            None,
        );
        Ok(())
    }

    /// Restore un-hides a hidden review and clears any flag. A review that is
    /// flagged but visible just has its flag cleared. A review that is neither
    /// fails with `ReviewNotHidden`.
    fn apply_restore_or_unflag(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        admin: &Address,
    ) -> Result<(), ContractError> {
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }
        let review: Review = env
            .storage()
            .persistent()
            .get(&StorageKey::Review(project_id, reviewer.clone()))
            .ok_or(ContractError::ReviewNotFound)?;
        let flagged = env
            .storage()
            .persistent()
            .has(&ReviewModerationKey::Flag(project_id, reviewer.clone()));

        if review.hidden {
            // Logs `ReviewRestored` and emits the restored event itself.
            Self::apply_restore_review(env, project_id, reviewer.clone(), admin.clone())?;
            Self::clear_flag(env, project_id, &reviewer);
            return Ok(());
        }
        if !flagged {
            return Err(ContractError::ReviewNotHidden);
        }

        Self::clear_flag(env, project_id, &reviewer);
        AdminActionLog::record_action(
            env,
            admin.clone(),
            AdminActionType::ReviewRestored,
            Some(project_id),
            Some(reviewer),
            None,
        );
        Ok(())
    }

    fn clear_flag(env: &Env, project_id: u64, reviewer: &Address) {
        env.storage()
            .persistent()
            .remove(&ReviewModerationKey::Flag(project_id, reviewer.clone()));
    }

    fn current_moderation_bucket(env: &Env) -> u64 {
        env.ledger().timestamp() / MODERATION_RATE_LIMIT_WINDOW_SECONDS
    }

    fn moderation_used(env: &Env, admin: &Address) -> u32 {
        let bucket = Self::current_moderation_bucket(env);
        env.storage()
            .persistent()
            .get::<_, ModerationWindow>(&ReviewModerationKey::RateWindow(admin.clone()))
            .filter(|window| window.bucket == bucket)
            .map(|window| window.count)
            .unwrap_or(0)
    }

    /// Charge `items` against the admin's window budget, rejecting the whole
    /// batch if it does not fit.
    fn consume_moderation_budget(
        env: &Env,
        admin: &Address,
        items: u32,
    ) -> Result<(), ContractError> {
        let used = Self::moderation_used(env, admin);
        let new_count = used.saturating_add(items);
        if new_count > MODERATION_ACTIONS_PER_WINDOW {
            return Err(ContractError::ModerationRateLimitExceeded);
        }

        let key = ReviewModerationKey::RateWindow(admin.clone());
        env.storage().persistent().set(
            &key,
            &ModerationWindow {
                bucket: Self::current_moderation_bucket(env),
                count: new_count,
            },
        );
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD_CRITICAL, LEDGER_BUMP_CRITICAL);
        Ok(())
    }
}
