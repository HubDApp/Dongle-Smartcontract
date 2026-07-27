use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::TIMELOCK_MIN_DELAY;
use crate::errors::ContractError;
use crate::events::{
    publish_timelock_action_cancelled_event, publish_timelock_action_executed_event,
    publish_timelock_action_scheduled_event,
};
use crate::fee_manager::FeeManager;
use crate::storage_keys::ExtensionKey;
use crate::types::{
    AdminActionType, TimelockAction, TimelockAdminAddParams, TimelockAdminRemoveParams,
    TimelockFeeParams,
};
use soroban_sdk::{Address, Env, Vec};

pub struct TimelockManager;

impl TimelockManager {
    fn get_next_id(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&ExtensionKey::NextTimelockActionId)
            .unwrap_or(1)
    }

    fn increment_id(env: &Env, id: u64) {
        env.storage()
            .persistent()
            .set(&ExtensionKey::NextTimelockActionId, &(id + 1));
    }

    fn save_action_ids(env: &Env, ids: &Vec<u64>) {
        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockActionIds, ids);
    }

    fn get_action_ids(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::TimelockActionIds)
            .unwrap_or(Vec::new(env))
    }

    fn validate_timelock(env: &Env, execution_timestamp: u64) {
        let now = env.ledger().timestamp();
        if execution_timestamp <= now {
            panic!("Timelock: execution timestamp must be in the future");
        }
        if execution_timestamp < now + TIMELOCK_MIN_DELAY {
            panic!("Timelock: minimum delay not met");
        }
    }

    fn get_action_unchecked(env: &Env, action_id: u64) -> TimelockAction {
        env.storage()
            .persistent()
            .get(&ExtensionKey::TimelockAction(action_id))
            .unwrap_or_else(|| panic!("Timelock: action not found"))
    }

    fn mark_executed(env: &Env, action_id: u64) {
        let mut action = Self::get_action_unchecked(env, action_id);
        action.executed = true;
        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockAction(action_id), &action);
    }

    fn require_pending(env: &Env, action_id: u64) -> TimelockAction {
        let action = Self::get_action_unchecked(env, action_id);
        if action.executed {
            panic!("Timelock: action already executed");
        }
        if action.cancelled {
            panic!("Timelock: action already cancelled");
        }
        action
    }

    fn require_expired(env: &Env, action: &TimelockAction) {
        let now = env.ledger().timestamp();
        if now < action.execution_timestamp {
            panic!("Timelock: action cannot execute before timelock expires");
        }
    }

    pub fn schedule_set_fee(
        env: &Env,
        admin: Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
        treasury: Address,
        execution_timestamp: u64,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;
        Self::validate_timelock(env, execution_timestamp);

        let now = env.ledger().timestamp();
        let id = Self::get_next_id(env);
        Self::increment_id(env, id);

        let action = TimelockAction {
            id,
            admin: admin.clone(),
            action_type: AdminActionType::FeeChanged,
            execution_timestamp,
            executed: false,
            cancelled: false,
            created_at: now,
        };

        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockAction(id), &action);

        let params = TimelockFeeParams {
            token,
            verification_fee,
            registration_fee,
            treasury,
        };
        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockFeeParams(id), &params);

        let mut ids = Self::get_action_ids(env);
        ids.push_back(id);
        Self::save_action_ids(env, &ids);

        publish_timelock_action_scheduled_event(
            env,
            id,
            admin,
            AdminActionType::FeeChanged,
            execution_timestamp,
        );

        Ok(id)
    }

    pub fn schedule_add_admin(
        env: &Env,
        admin: Address,
        new_admin: Address,
        execution_timestamp: u64,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;
        Self::validate_timelock(env, execution_timestamp);

        let now = env.ledger().timestamp();
        let id = Self::get_next_id(env);
        Self::increment_id(env, id);

        let action = TimelockAction {
            id,
            admin: admin.clone(),
            action_type: AdminActionType::AdminAdded,
            execution_timestamp,
            executed: false,
            cancelled: false,
            created_at: now,
        };

        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockAction(id), &action);

        let params = TimelockAdminAddParams { new_admin };
        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockAdminAddParams(id), &params);

        let mut ids = Self::get_action_ids(env);
        ids.push_back(id);
        Self::save_action_ids(env, &ids);

        publish_timelock_action_scheduled_event(
            env,
            id,
            admin,
            AdminActionType::AdminAdded,
            execution_timestamp,
        );

        Ok(id)
    }

    pub fn schedule_remove_admin(
        env: &Env,
        admin: Address,
        admin_to_remove: Address,
        execution_timestamp: u64,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;
        Self::validate_timelock(env, execution_timestamp);

        let now = env.ledger().timestamp();
        let id = Self::get_next_id(env);
        Self::increment_id(env, id);

        let action = TimelockAction {
            id,
            admin: admin.clone(),
            action_type: AdminActionType::AdminRemoved,
            execution_timestamp,
            executed: false,
            cancelled: false,
            created_at: now,
        };

        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockAction(id), &action);

        let params = TimelockAdminRemoveParams { admin_to_remove };
        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockAdminRemoveParams(id), &params);

        let mut ids = Self::get_action_ids(env);
        ids.push_back(id);
        Self::save_action_ids(env, &ids);

        publish_timelock_action_scheduled_event(
            env,
            id,
            admin,
            AdminActionType::AdminRemoved,
            execution_timestamp,
        );

        Ok(id)
    }

    pub fn cancel_action(env: &Env, caller: Address, action_id: u64) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        let action = Self::require_pending(env, action_id);

        let mut updated = action.clone();
        updated.cancelled = true;
        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockAction(action_id), &updated);

        publish_timelock_action_cancelled_event(env, action_id, caller, action.action_type);

        Ok(())
    }

    pub fn execute_set_fee(
        env: &Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        let action = Self::require_pending(env, action_id);
        Self::require_expired(env, &action);

        let params: TimelockFeeParams = env
            .storage()
            .persistent()
            .get(&ExtensionKey::TimelockFeeParams(action_id))
            .expect("Timelock: fee params not found");

        FeeManager::set_fee(
            env,
            action.admin.clone(),
            params.token,
            params.verification_fee,
            params.registration_fee,
            params.treasury,
        )?;

        Self::mark_executed(env, action_id);

        publish_timelock_action_executed_event(env, action_id, caller, AdminActionType::FeeChanged);

        Ok(())
    }

    pub fn execute_add_admin(
        env: &Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        let action = Self::require_pending(env, action_id);
        Self::require_expired(env, &action);

        let params: TimelockAdminAddParams = env
            .storage()
            .persistent()
            .get(&ExtensionKey::TimelockAdminAddParams(action_id))
            .expect("Timelock: admin add params not found");

        AdminManager::add_admin(env, action.admin.clone(), params.new_admin)?;

        Self::mark_executed(env, action_id);

        publish_timelock_action_executed_event(env, action_id, caller, AdminActionType::AdminAdded);

        Ok(())
    }

    pub fn execute_remove_admin(
        env: &Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        let action = Self::require_pending(env, action_id);
        Self::require_expired(env, &action);

        let params: TimelockAdminRemoveParams = env
            .storage()
            .persistent()
            .get(&ExtensionKey::TimelockAdminRemoveParams(action_id))
            .expect("Timelock: admin remove params not found");

        AdminManager::remove_admin(env, action.admin.clone(), params.admin_to_remove)?;

        Self::mark_executed(env, action_id);

        publish_timelock_action_executed_event(
            env,
            action_id,
            caller,
            AdminActionType::AdminRemoved,
        );

        Ok(())
    }

    pub fn get_action(env: &Env, action_id: u64) -> Option<TimelockAction> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::TimelockAction(action_id))
    }

    pub fn list_scheduled_actions(env: &Env, start: u32, limit: u32) -> Vec<TimelockAction> {
        let ids = Self::get_action_ids(env);
        let total = ids.len();

        if total == 0 || start >= total {
            return Vec::new(env);
        }

        let end = (start + limit).min(total);
        let mut actions = Vec::new(env);

        for i in start..end {
            if let Some(id) = ids.get(i) {
                if let Some(action) = Self::get_action(env, id) {
                    actions.push_back(action);
                }
            }
        }

        actions
    }

    pub fn get_scheduled_action_count(env: &Env) -> u64 {
        let ids = Self::get_action_ids(env);
        ids.len() as u64
    }
}
