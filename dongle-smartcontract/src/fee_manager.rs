//! Fee configuration and payment with validation and events.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::events::{publish_fee_paid_event, publish_fee_set_event};
use crate::storage_keys::StorageKey;
use crate::types::FeeConfig;
use soroban_sdk::{Address, Env};

pub struct FeeManager;

impl FeeManager {
    /// Configure fees for the contract (admin only)
    pub fn set_fee(
        env: &Env,
        admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;

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

    /// Pay the verification fee for a project
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
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)?;

        if config.token != token {
            return Err(ContractError::InvalidProjectData);
        }

        let amount = config.verification_fee;
        if amount > 0 {
            let token_address = config.token.ok_or(ContractError::FeeConfigNotSet)?;
            let client = soroban_sdk::token::Client::new(env, &token_address);
            client.transfer(&payer, &treasury, &(amount as i128));
        }

        env.storage()
            .persistent()
            .set(&StorageKey::FeePaidForProject(project_id), &true);

        publish_fee_paid_event(env, project_id, amount);
        Ok(())
    }

    /// Check if the fee has been paid for a project
    pub fn is_fee_paid(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::FeePaidForProject(project_id))
            .unwrap_or(false)
    }

    /// Consume the fee payment (used during verification request)
    pub fn consume_fee_payment(env: &Env, project_id: u64) -> Result<(), ContractError> {
        if !Self::is_fee_paid(env, project_id) {
            return Err(ContractError::InsufficientFee);
        }
        env.storage()
            .persistent()
            .remove(&StorageKey::FeePaidForProject(project_id));
        Ok(())
    }

    /// Get current fee configuration
    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::FeeConfig)
            .ok_or(ContractError::FeeConfigNotSet)
    }

    /// Set the treasury address (admin only)
    pub fn set_treasury(env: &Env, admin: Address, treasury: Address) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;

        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);
        Ok(())
    }

    /// Get the current treasury address
    pub fn get_treasury(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)
    }

    /// Get fee for a specific operation
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
