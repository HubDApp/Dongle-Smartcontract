//! Fee configuration and payment with validation and events.

use crate::auth::{require_admin_auth, require_self_auth};
use crate::errors::ContractError;
use crate::events::{publish_fee_paid_event, publish_fee_set_event};
use crate::project_registry::ProjectRegistry;
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
        verification_fee: u128,
        registration_fee: u128,
        verification_duration: u64,
        treasury: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let config = FeeConfig {
            token,
            verification_fee,
            registration_fee,
            verification_duration,
        };
        env.storage()
            .persistent()
            .set(&StorageKey::FeeConfig, &config);
        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);

        publish_fee_set_event(env, verification_fee, registration_fee);
        Ok(())
    }

    /// Pay the verification fee for a project.
    /// Only the project owner may pay; third-party payments are rejected.
    pub fn pay_fee(
        env: &Env,
        payer: Address,
        project_id: u64,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        require_self_auth(&payer);

        // Enforce owner-only payment
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.owner != payer {
            return Err(ContractError::Unauthorized);
        }

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

        publish_fee_paid_event(env, project_id, payer, amount);
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
        let config = Self::get_fee_config(env)?;
        // If verification fee is zero, nothing to consume
        if config.verification_fee == 0u128 {
            return Ok(());
        }
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
        // If fee configuration is not set, return a default zeroed config
        // to preserve backward compatibility with tests and calls that
        // expect no fees by default.
        let default = FeeConfig {
            token: None,
            verification_fee: 0u128,
            registration_fee: 0u128,
            verification_duration: 0u64,
        };
        Ok(env
            .storage()
            .persistent()
            .get(&StorageKey::FeeConfig)
            .unwrap_or(default))
    }

    /// Set the treasury address (admin only)
    #[allow(dead_code)]
    pub fn set_treasury(env: &Env, admin: Address, treasury: Address) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);
        Ok(())
    }

    /// Get the current treasury address
    #[allow(dead_code)]
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

    /// Pay the registration fee for a project.
    /// Only the project owner may pay; third-party payments are rejected.
    pub fn pay_registration_fee(
        env: &Env,
        payer: Address,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        require_self_auth(&payer);

        let config = Self::get_fee_config(env)?;
        let treasury: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)?;

        if config.token != token {
            return Err(ContractError::InvalidProjectData);
        }

        let amount = config.registration_fee;
        if amount > 0 {
            let token_address = config.token.ok_or(ContractError::FeeConfigNotSet)?;
            let client = soroban_sdk::token::Client::new(env, &token_address);
            client.transfer(&payer, &treasury, &(amount as i128));
        }

        env.storage()
            .persistent()
            .set(&StorageKey::RegistrationFeePaidForAddress(payer.clone()), &true);

        publish_fee_paid_event(env, 0, payer, amount);
        Ok(())
    }

    /// Check if the registration fee has been paid for an address
    pub fn is_registration_fee_paid(env: &Env, address: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::RegistrationFeePaidForAddress(address.clone()))
            .unwrap_or(false)
    }

    /// Consume the registration fee payment (used during project registration)
    pub fn consume_registration_fee_payment(env: &Env, address: &Address) -> Result<(), ContractError> {
        let config = Self::get_fee_config(env)?;
        // If registration fee is zero, nothing to consume
        if config.registration_fee == 0u128 {
            return Ok(());
        }
        if !Self::is_registration_fee_paid(env, address) {
            return Err(ContractError::InsufficientFee);
        }
        env.storage()
            .persistent()
            .remove(&StorageKey::RegistrationFeePaidForAddress(address.clone()));
        Ok(())
    }
}
