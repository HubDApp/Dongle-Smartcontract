//! Fee configuration and payment with validation and events.

use crate::admin_registry::AdminRegistry;
use crate::errors::ContractError;
use crate::events::{
    publish_fee_paid_event, publish_fee_set_event, publish_treasury_updated_event,
};
use crate::types::{DataKey, FeeConfig};
use soroban_sdk::{Address, Env};

pub struct FeeManager;

impl FeeManager {
    pub fn set_fee(
        env: &Env,
        admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        // Check admin authorization
        AdminRegistry::require_admin(env, &admin)?;

        // Validate fee amount
        if amount == 0 {
            return Err(ContractError::InvalidFeeAmount);
        }

        // Create fee configuration
        let fee_config = FeeConfig {
            token,
            verification_fee: amount,
            registration_fee: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::FeeConfig, &fee_config);
        env.storage()
            .persistent()
            .set(&DataKey::Treasury, &treasury);

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

        // Get fee configuration
        let fee_config = Self::get_fee_config(env)?;

        // Validate token matches configuration
        if fee_config.token != token {
            return Err(ContractError::InvalidFeeAmount);
        }

        // In a real implementation, this would transfer tokens
        // For now, we just record that the fee was paid

        publish_fee_paid_event(env, project_id, fee_config.verification_fee);

        Ok(())
    }

    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::FeeConfig)
            .ok_or(ContractError::FeeConfigNotSet)
    }

    pub fn set_treasury(env: &Env, admin: Address, treasury: Address) -> Result<(), ContractError> {
        admin.require_auth();

        // Check admin authorization
        AdminRegistry::require_admin(env, &admin)?;

        env.storage()
            .persistent()
            .set(&DataKey::Treasury, &treasury);
        publish_treasury_updated_event(env, admin, treasury);

        Ok(())
    }

    pub fn get_treasury(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)
    }

    pub fn get_operation_fee(_env: &Env, operation_type: &str) -> Result<u128, ContractError> {
        match operation_type {
            "verification" => Ok(1000000),
            "registration" => Ok(0),
            _ => Err(ContractError::InvalidProjectData),
        }
    }

    pub fn fee_config_exists(env: &Env) -> bool {
        env.storage().persistent().has(&DataKey::FeeConfig)
    }

    pub fn treasury_exists(env: &Env) -> bool {
        env.storage().persistent().has(&DataKey::Treasury)
    }

    pub fn refund_fee(
        env: &Env,
        admin: Address,
        recipient: Address,
        amount: u128,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        // Check admin authorization
        AdminRegistry::require_admin(env, &admin)?;

        // In a real implementation, this would transfer tokens back
        // For now, we just validate the operation is allowed

        Ok(())
    }
}
