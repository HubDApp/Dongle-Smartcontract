use crate::errors::ContractError;
use crate::events::{publish_fee_collected_event, publish_treasury_withdrawal_event};
use crate::storage_keys::StorageKey;
use crate::types::FeeConfig;
use soroban_sdk::{token, Address, Env};

pub struct FeeManager;

impl FeeManager {
    pub fn set_fee(
        env: &Env,
        admin: Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        
        // Only existing admin can set fee if admin exists
        if let Some(current_admin) = env.storage().persistent().get::<_, Address>(&StorageKey::Admin) {
            if current_admin != admin {
                return Err(ContractError::Unauthorized);
            }
        }

        let config = FeeConfig {
            token,
            verification_fee,
            registration_fee,
        };

        env.storage().persistent().set(&StorageKey::FeeConfig, &config);
        Ok(())
    }

    pub fn pay_fee(
        env: &Env,
        payer: Address,
        project_id: u64,
        operation_type: &str,
    ) -> Result<(), ContractError> {
        payer.require_auth();

        let config = Self::get_fee_config(env)?;
        let amount = match operation_type {
            "verification" => config.verification_fee,
            "registration" => config.registration_fee,
            _ => return Err(ContractError::InvalidProjectData),
        };

        if amount == 0 {
            return Ok(());
        }

        let token_addr = config.token.ok_or(ContractError::FeeNotConfigured)?;
        let client = token::Client::new(env, &token_addr);
        
        // Transfer from payer to contract
        client.transfer(&payer, &env.current_contract_address(), &(amount as i128));

        // Update treasury balance in storage
        let mut balance = Self::get_treasury_balance(env, &token_addr);
        balance = balance.saturating_add(amount);
        env.storage().persistent().set(&StorageKey::TreasuryBalance(token_addr.clone()), &balance);

        // Mark fee as paid for project if verification
        if operation_type == "verification" {
            env.storage().persistent().set(&StorageKey::FeePaidForProject(project_id), &true);
        }

        publish_fee_collected_event(env, payer, project_id, token_addr, amount);

        Ok(())
    }

    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::FeeConfig)
            .ok_or(ContractError::FeeConfigNotSet)
    }

    pub fn withdraw_treasury(
        env: &Env,
        admin: Address,
        token_addr: Address,
        amount: u128,
        to: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        let stored_admin: Address = env.storage().persistent().get(&StorageKey::Admin).ok_or(ContractError::AdminOnly)?;
        if admin != stored_admin {
            return Err(ContractError::AdminOnly);
        }

        let mut balance = Self::get_treasury_balance(env, &token_addr);
        if balance < amount {
            return Err(ContractError::InsufficientBalance);
        }

        let client = token::Client::new(env, &token_addr);
        client.transfer(&env.current_contract_address(), &to, &(amount as i128));

        balance = balance.saturating_sub(amount);
        env.storage().persistent().set(&StorageKey::TreasuryBalance(token_addr.clone()), &balance);

        publish_treasury_withdrawal_event(env, token_addr, amount, to);

        Ok(())
    }

    pub fn get_treasury_balance(env: &Env, token: &Address) -> u128 {
        env.storage()
            .persistent()
            .get(&StorageKey::TreasuryBalance(token.clone()))
            .unwrap_or(0)
    }

    pub fn set_admin(env: &Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        if let Some(current_admin) = env.storage().persistent().get::<_, Address>(&StorageKey::Admin) {
            caller.require_auth();
            if caller != current_admin {
                return Err(ContractError::AdminOnly);
            }
        }
        // If no admin, anyone can initialize? Or handled in initialize() in lib.rs
        env.storage().persistent().set(&StorageKey::Admin, &new_admin);
        Ok(())
    }

    pub fn is_fee_paid(env: &Env, project_id: u64) -> bool {
        env.storage().persistent().get(&StorageKey::FeePaidForProject(project_id)).unwrap_or(false)
    }
}
