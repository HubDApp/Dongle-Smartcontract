//! Fee configuration and payment with validation and events.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::events::{publish_fee_paid_event, publish_fee_set_event};
use crate::storage_keys::StorageKey;
use crate::types::FeeConfig;
use soroban_sdk::{Address, Env};

pub struct FeeManager;

#[allow(dead_code)]
impl FeeManager {
    pub fn set_fee(
        env: &Env,
        admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        // Verify admin privileges
        AdminManager::require_admin(env, &admin)?;

        // Authorization check
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Admin)
            .ok_or(ContractError::Unauthorized)?;
        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        let config = FeeConfig {
            token,
            verification_fee: amount,
            registration_fee: 0,
        };
        env.storage()
            .persistent()
            .set(&StorageKey::FeeConfig, &config);
        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);
        publish_fee_set_event(env, amount, 0);
        Ok(())
    }

    pub fn pay_fee(
        env: &Env,
        payer: Address,
        project_id: u64,

      token: Option<Address>,
    ) -> Result<(), ContractError> {
        payer.require_auth();

        let config = Self::get_fee_config(env)?;
        let treasury: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)?;

        if config.token != token {
            return Err(ContractError::InvalidProjectData);
        }

        let amount = config.verification_fee;
        if amount > 0 {
            if let Some(token_address) = config.token {
                let client = soroban_sdk::token::Client::new(env, &token_address);
                client.transfer(&payer, &treasury, &(amount as i128));
            } else {
                // For native token, we use the same token client since it's standardized
                // Assuming the contract environment has access to the native asset if token is None
                // In Soroban, native asset is also a token. 
                // However, the FeeConfig doesn't store the native asset address if None.
                // We'll require the caller to pass the correct token address if it's not None.
                // If config.token is None, it means the contract isn't fully configured for native payments yet 
                // or we need a standard way to get the native asset address.
                return Err(ContractError::FeeConfigNotSet);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::FeePaidForProject(project_id), &true);

        crate::events::publish_fee_paid_event(env, project_id, amount);

        Ok(())
    }

    pub fn is_fee_paid(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::FeePaidForProject(project_id))
            .unwrap_or(false)
    }

    pub fn consume_fee_payment(env: &Env, project_id: u64) -> Result<(), ContractError> {
        if !Self::is_fee_paid(env, project_id) {
            return Err(ContractError::InsufficientFee);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::FeePaidForProject(project_id));
        Ok(())
    }

        env.storage()
            .persistent()
            .set(&StorageKey::FeePaidForProject(project_id), &true);

        publish_fee_paid_event(env, project_id, config.verification_fee);
        Ok(())
    }

    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::FeeConfig)
            .ok_or(ContractError::FeeConfigNotSet)
    }

    pub fn set_treasury(env: &Env, admin: Address, treasury: Address) -> Result<(), ContractError> {
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Admin)
            .ok_or(ContractError::Unauthorized)?;
        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);
        Ok(())
    }

    pub fn get_treasury(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::FeeConfigNotSet) // Reusing error or could use a new one
    }

    pub fn is_fee_paid(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::FeePaidForProject(project_id))
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn get_operation_fee(env: &Env, operation_type: &str) -> Result<u128, ContractError> {
        let config = Self::get_fee_config(env)?;
        match operation_type {
            "verification" => Ok(config.verification_fee),
            "registration" => Ok(config.registration_fee),
            _ => Err(ContractError::InvalidProjectData),
        }
    }
}
