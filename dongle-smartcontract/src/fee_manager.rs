//! Fee configuration and payment with validation and events.
use crate::errors::ContractError;
use crate::events::FeePaid;
use crate::events::FeeSet;
use crate::storage_keys::StorageKey;
use crate::types::FeeConfig;
use crate::verification_registry::VerificationRegistry;
use soroban_sdk::{Address, Env};

pub struct FeeManager;

impl FeeManager {
    pub fn set_admin(env: &Env, admin: Address) {
        env.storage().persistent().set(&StorageKey::Admin, &admin);
    }

    /// Sets fee config with separate verification and registration fees.
    /// Called from lib.rs set_fee_config.
    pub fn set_fee_config(
        env: &Env,
        admin: &Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        let current_admin: Option<Address> = env.storage().persistent().get(&StorageKey::Admin);
        if current_admin.as_ref() != Some(admin) {
            return Err(ContractError::UnauthorizedAdmin);
        }
        if verification_fee == 0 && registration_fee == 0 {
            return Err(ContractError::InvalidFeeAmount);
        }
        let treasury: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::InvalidTreasury)?;

        let config = FeeConfig {
            token,
            amount: verification_fee,
            treasury: treasury.clone(),
        };
        env.storage()
            .persistent()
            .set(&StorageKey::FeeConfig, &config);

        FeeSet {
            admin: admin.clone(),
            amount: verification_fee,
            treasury,
        }
        .publish(env);
        Ok(())
    }

    /// Legacy single-fee setter (used internally).
    pub fn set_fee(
        env: &Env,
        admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        let current_admin: Option<Address> = env.storage().persistent().get(&StorageKey::Admin);
        if current_admin.as_ref() != Some(&admin) {
            return Err(ContractError::UnauthorizedAdmin);
        }
        if amount == 0 {
            return Err(ContractError::InvalidFeeAmount);
        }
        let config = FeeConfig {
            token,
            amount,
            treasury: treasury.clone(),
        };
        env.storage()
            .persistent()
            .set(&StorageKey::FeeConfig, &config);
        FeeSet {
            admin,
            amount,
            treasury,
        }
        .publish(env);
        Ok(())
    }

    fn get_config(env: &Env) -> Result<FeeConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::FeeConfig)
            .ok_or(ContractError::FeeNotConfigured)
    }

    pub fn pay_fee(
        env: &Env,
        payer: Address,
        project_id: u64,
        _token: Option<Address>,
    ) -> Result<(), ContractError> {
        let config = Self::get_config(env)?;
        if config.amount == 0 {
            return Err(ContractError::InvalidFeeAmount);
        }
        VerificationRegistry::set_fee_paid(env, project_id);
        FeePaid {
            payer: payer.clone(),
            project_id,
            amount: config.amount,
        }
        .publish(env);
        Ok(())
    }

    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, ContractError> {
        Self::get_config(env)
    }

    /// Sets the treasury address. Caller must be admin.
    pub fn set_treasury(
        env: &Env,
        admin: &Address,
        treasury: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        let current_admin: Option<Address> = env.storage().persistent().get(&StorageKey::Admin);
        if current_admin.as_ref() != Some(admin) {
            return Err(ContractError::UnauthorizedAdmin);
        }
        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);
        Ok(())
    }

    /// Returns the current treasury address.
    pub fn get_treasury(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::InvalidTreasury)
    }
}
