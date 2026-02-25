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

    pub fn set_fee(
        env: &Env,
        admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), Error> {
        let current_admin: Option<Address> = env.storage().persistent().get(&StorageKey::Admin);
        if current_admin.as_ref() != Some(&admin) {
            return Err(Error::UnauthorizedAdmin);
        }
        if amount == 0 {
            return Err(Error::InvalidFeeAmount);
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

    fn get_config(env: &Env) -> Result<FeeConfig, Error> {
        env.storage()
            .persistent()
            .get(&StorageKey::FeeConfig)
            .ok_or(Error::FeeNotConfigured)
    }

    /// Pay verification fee for a project. In a full implementation this would transfer
    /// tokens to treasury; here we record payment and mark fee as paid for verification.
    /// Contract is modular: replace this with real token transfer when integrating.
    pub fn pay_fee(
        env: &Env,
        payer: Address,
        project_id: u64,
        _token: Option<Address>,
    ) -> Result<(), Error> {
        let config = Self::get_config(env)?;

        // Simulated transfer: in production, env.invoke_contract() to token transfer
        // from payer to config.treasury for config.amount. On failure return PaymentFailed.
        // For now we require amount > 0 and record the payment.
        if config.amount == 0 {
            return Err(Error::InvalidFeeAmount);
        }

        // Mark fee as paid for this project so verification can proceed.
        VerificationRegistry::set_fee_paid(env, project_id);

        FeePaid {
            payer: payer.clone(),
            project_id,
            amount: config.amount,
        }
        .publish(env);

        Ok(())
    }

    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, Error> {
        Self::get_config(env)
    }
}
