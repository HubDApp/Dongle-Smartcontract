//! Fee configuration and payment with validation and events.

use crate::errors::ContractError;
use crate::events::{publish_fee_set_event, publish_fee_paid_event};
use crate::types::FeeConfig;
use crate::storage_keys::StorageKey;
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
        // Authorization check
        let stored_admin: Address = env.storage().persistent().get(&StorageKey::Admin)
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
        env.storage().persistent().set(&StorageKey::FeeConfig, &config);
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
        _token: Option<Address>,
    ) -> Result<(), ContractError> {
        payer.require_auth();

        let config = Self::get_fee_config(env)?;
        let treasury = Self::get_treasury(env)?;

        if config.verification_fee > 0 {
            if let Some(token_address) = config.token {
                let client = soroban_sdk::token::Client::new(env, &token_address);
                client.transfer(&payer, &treasury, &(config.verification_fee as i128));
            } else {
                 // Native XLM transfer not directly supported in this simple way via token client if it's not a token address
                 // Assuming token address is provided for now as per implementation plan.
                 return Err(ContractError::InvalidProjectData);
            }
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

    pub fn set_treasury(
        env: &Env,
        admin: Address,
        treasury: Address,
    ) -> Result<(), ContractError> {
        let stored_admin: Address = env.storage().persistent().get(&StorageKey::Admin)
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
