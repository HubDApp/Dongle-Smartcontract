//! Fee configuration and payment with validation and events.

use crate::errors::ContractError;
use crate::events::publish_fee_set_event;
use crate::types::{DataKey, FeeConfig};
use soroban_sdk::{Address, Env};

pub struct FeeManager;

impl FeeManager {
    pub fn set_fee(
        env: &Env,
        _admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        let config = FeeConfig {
            token,
            verification_fee: amount,
            registration_fee: 0,
        };
        env.storage().persistent().set(&DataKey::FeeConfig, &config);
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

    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::FeeConfig)
            .ok_or(ContractError::FeeConfigNotSet)
    }

    #[allow(dead_code)]
    pub fn set_treasury(
        _env: &Env,
        _admin: Address,
        _treasury: Address,
    ) -> Result<(), ContractError> {
        todo!("Treasury setting logic not implemented")
    }

    #[allow(dead_code)]
    pub fn get_treasury(_env: &Env) -> Result<Address, ContractError> {
        todo!("Treasury address retrieval logic not implemented")
    }

    #[allow(dead_code)]
    pub fn get_operation_fee(_env: &Env, operation_type: &str) -> Result<u128, ContractError> {
        match operation_type {
            "verification" => Ok(1000000),
            "registration" => Ok(0),
            _ => Err(ContractError::InvalidProjectData),
        }
    }

    #[allow(dead_code)]
    pub fn fee_config_exists(_env: &Env) -> bool {
        false
    }

    #[allow(dead_code)]
    pub fn treasury_exists(_env: &Env) -> bool {
        false
    }

    #[allow(dead_code)]
    pub fn refund_fee(
        _env: &Env,
        _recipient: Address,
        _amount: u128,
        _token: Option<Address>,
    ) -> Result<(), ContractError> {
        todo!("Fee refund logic not implemented")
    }
}
