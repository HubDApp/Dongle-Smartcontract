//! Fee configuration and payment with validation and events.

use crate::admin_action_log::AdminActionLog;
use crate::auth::{require_admin_auth, require_self_auth};
use crate::errors::ContractError;
use crate::events::{
    publish_fee_consumed_event, publish_fee_paid_event, publish_fee_set_event, FeeOperation,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::types::{AdminActionType, FeeConfig, FeePaymentRecord};
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
        treasury: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if crate::admin_manager::AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::Unauthorized);
        }

        let config = FeeConfig {
            token,
            verification_fee,
            registration_fee,
        };
        env.storage()
            .persistent()
            .set(&StorageKey::FeeConfig, &config);
        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);

        publish_fee_set_event(
            env,
            admin.clone(),
            config.token.clone(),
            verification_fee,
            registration_fee,
            treasury,
        );

        AdminActionLog::record_action(env, admin, AdminActionType::FeeChanged, None, None, None);

        Ok(())
    }

    /// Pay the verification fee for a project.
    /// Only the project owner may pay; third-party payments are rejected.
    /// 
    /// # Behavior on Token Transfer Failure
    /// - If the token transfer fails (e.g., insufficient balance), the payment flag is NOT set
    /// - The fee paid event is NOT emitted
    /// - The caller receives an error and can retry after acquiring sufficient tokens
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
            // set_fee enforces that token is Some when fees are non-zero, so this
            // ok_or branch is a defensive guard against corrupted storage state.
            let token_address = config.token.ok_or(ContractError::NativeFeeNotSupported)?;
            let client = soroban_sdk::token::Client::new(env, &token_address);
            // Transfer must succeed before we set the payment flag.
            // If transfer fails, this function returns early without setting the flag.
            client.transfer(&payer, &treasury, &(amount as i128));
        }

        // Only set payment flag after successful token transfer
        env.storage()
            .persistent()
            .set(&StorageKey::FeePaidForProject(project_id), &true);

        // Store full payment details for getter
        let payment_record = FeePaymentRecord {
            paid_at: env.ledger().timestamp(),
            payer: payer.clone(),
            amount,
            token: token.clone(),
        };
        env.storage().persistent().set(
            &ExtensionKey::FeePaymentDetails(project_id),
            &payment_record,
        );

        // Only emit event after successful payment
        publish_fee_paid_event(
            env,
            project_id,
            payer,
            token,
            FeeOperation::Verification,
            amount,
        );
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
    pub fn consume_fee_payment(
        env: &Env,
        project_id: u64,
        caller: Address,
        amount: u128,
    ) -> Result<(), ContractError> {
        if !Self::is_fee_paid(env, project_id) {
            return Err(ContractError::InsufficientFee);
        }
        env.storage()
            .persistent()
            .remove(&StorageKey::FeePaidForProject(project_id));
        publish_fee_consumed_event(env, project_id, caller, FeeOperation::Verification, amount);
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
    ///
    /// # Behavior on Token Transfer Failure
    /// - If the token transfer fails (e.g., insufficient balance), the payment flag is NOT set
    /// - The fee paid event is NOT emitted
    /// - The caller receives an error and can retry after acquiring sufficient tokens
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
            // Defensive guard — set_fee already rejects None token with non-zero fees.
            let token_address = config.token.ok_or(ContractError::NativeFeeNotSupported)?;
            let client = soroban_sdk::token::Client::new(env, &token_address);
            // Transfer must succeed before we set the payment flag.
            // If transfer fails, this function returns early without setting the flag.
            client.transfer(&payer, &treasury, &(amount as i128));
        }

        // Only set payment flag after successful token transfer
        env.storage()
            .persistent()
            .set(&StorageKey::RegistrationFeePaidForAddress(payer.clone()), &true);

        // Store full payment details for getter
        let payment_record = FeePaymentRecord {
            paid_at: env.ledger().timestamp(),
            payer: payer.clone(),
            amount,
            token: token.clone(),
        };
        env.storage().persistent().set(
            &ExtensionKey::RegistrationFeePaymentDetails(payer.clone()),
            &payment_record,
        );

        // Only emit event after successful payment
        publish_fee_paid_event(env, 0, payer, token, FeeOperation::Registration, amount);
        Ok(())
    }

    /// Get fee payment details for a project (payer, amount, token, timestamp)
    pub fn get_fee_payment_details(env: &Env, project_id: u64) -> Option<FeePaymentRecord> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::FeePaymentDetails(project_id))
    }

    /// Get registration fee payment details for an address
    pub fn get_registration_fee_payment_details(
        env: &Env,
        address: &Address,
    ) -> Option<FeePaymentRecord> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::RegistrationFeePaymentDetails(address.clone()))
    }

    /// Check if the registration fee has been paid for an address
    pub fn is_registration_fee_paid(env: &Env, address: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::RegistrationFeePaidForAddress(address.clone()))
            .unwrap_or(false)
    }

    /// Consume the registration fee payment (used during project registration)
    pub fn consume_registration_fee_payment(
        env: &Env,
        address: &Address,
        amount: u128,
    ) -> Result<(), ContractError> {
        if !Self::is_registration_fee_paid(env, address) {
            return Err(ContractError::InsufficientFee);
        }
        env.storage()
            .persistent()
            .remove(&StorageKey::RegistrationFeePaidForAddress(address.clone()));
        publish_fee_consumed_event(env, 0, address.clone(), FeeOperation::Registration, amount);
        Ok(())
    }
}
