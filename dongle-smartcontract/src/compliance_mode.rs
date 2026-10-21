//! Restricted-mode compliance deployments (#947).
//!
//! Some jurisdictions require feature restrictions. When compliance mode is
//! enabled by an admin:
//!
//! - anonymous reviews and anonymous follows are rejected,
//! - projects must carry a geographic location (country code) requirement,
//! - KYC/AML integration hooks must approve regulated actors,
//! - every compliance decision is appended to an on-chain audit log.

use soroban_sdk::{contracttype, Address, Env, String, Vec};

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceConfig {
    pub enabled: bool,
    pub require_geo: bool,
    pub require_kyc: bool,
    pub allow_anonymous_reviews: bool,
    pub allow_anonymous_follows: bool,
}

impl Default for ComplianceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            require_geo: false,
            require_kyc: false,
            allow_anonymous_reviews: true,
            allow_anonymous_follows: true,
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceAuditEntry {
    pub id: u64,
    pub actor: Address,
    pub action: String,
    pub detail: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum ComplianceKey {
    Config,
    AuditLog,
    AuditCount,
    KycApproved(Address),
    ProjectGeo(u64),
}

pub struct ComplianceMode;

impl ComplianceMode {
    pub fn get_config(env: &Env) -> ComplianceConfig {
        env.storage()
            .persistent()
            .get(&ComplianceKey::Config)
            .unwrap_or(ComplianceConfig::default())
    }

    /// Admin-only: enable/disable restricted mode and its sub-features.
    pub fn set_config(
        env: &Env,
        admin: Address,
        config: ComplianceConfig,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        env.storage()
            .persistent()
            .set(&ComplianceKey::Config, &config);
        Self::audit(env, admin, String::from_str(env, "config_updated"), String::from_str(env, "compliance config updated"));
        Ok(())
    }

    pub fn is_enabled(env: &Env) -> bool {
        Self::get_config(env).enabled
    }

    /// Gate anonymous reviews: rejected when compliance mode forbids them.
    pub fn require_anonymous_review_allowed(env: &Env) -> Result<(), ContractError> {
        let cfg = Self::get_config(env);
        if cfg.enabled && !cfg.allow_anonymous_reviews {
            return Err(ContractError::Unauthorized);
        }
        Ok(())
    }

    /// Gate anonymous follows: rejected when compliance mode forbids them.
    pub fn require_anonymous_follow_allowed(env: &Env) -> Result<(), ContractError> {
        let cfg = Self::get_config(env);
        if cfg.enabled && !cfg.allow_anonymous_follows {
            return Err(ContractError::Unauthorized);
        }
        Ok(())
    }

    /// Geographic requirement: projects must declare a country code when
    /// `require_geo` is set. Owners/admins set it at registration or later.
    pub fn set_project_geo(
        env: &Env,
        caller: Address,
        project_id: u64,
        country_code: String,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        if country_code.len() == 0 || country_code.len() > 8 {
            return Err(ContractError::InvalidInput);
        }
        env.storage()
            .persistent()
            .set(&ComplianceKey::ProjectGeo(project_id), &country_code);
        Self::audit(env, caller, String::from_str(env, "geo_set"), country_code);
        Ok(())
    }

    pub fn get_project_geo(env: &Env, project_id: u64) -> Option<String> {
        env.storage()
            .persistent()
            .get(&ComplianceKey::ProjectGeo(project_id))
    }

    /// Enforce geo requirement before project registration completes.
    pub fn require_geo_satisfied(env: &Env, project_id: u64) -> Result<(), ContractError> {
        let cfg = Self::get_config(env);
        if cfg.enabled && cfg.require_geo && Self::get_project_geo(env, project_id).is_none() {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    // ── KYC/AML hooks ──────────────────────────────────────────────

    /// Admin-only hook for an off-chain KYC/AML provider to approve `user`.
    /// Regulated flows call `require_kyc_approved` before proceeding.
    pub fn set_kyc_approved(
        env: &Env,
        admin: Address,
        user: Address,
        approved: bool,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        env.storage()
            .persistent()
            .set(&ComplianceKey::KycApproved(user.clone()), &approved);
        let detail = if approved {
            String::from_str(env, "kyc approved")
        } else {
            String::from_str(env, "kyc revoked")
        };
        Self::audit(env, admin, String::from_str(env, "kyc_decision"), detail);
        let _ = user;
        Ok(())
    }

    pub fn is_kyc_approved(env: &Env, user: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&ComplianceKey::KycApproved(user.clone()))
            .unwrap_or(false)
    }

    /// Enforce KYC when compliance mode requires it.
    pub fn require_kyc_approved(env: &Env, user: &Address) -> Result<(), ContractError> {
        let cfg = Self::get_config(env);
        if cfg.enabled && cfg.require_kyc && !Self::is_kyc_approved(env, user) {
            return Err(ContractError::Unauthorized);
        }
        Ok(())
    }

    // ── Audit log ──────────────────────────────────────────────────

    fn audit(env: &Env, actor: Address, action: String, detail: String) {
        let next: u64 = env
            .storage()
            .persistent()
            .get(&ComplianceKey::AuditCount)
            .unwrap_or(0);
        let id = next.saturating_add(1);
        env.storage()
            .persistent()
            .set(&ComplianceKey::AuditCount, &id);
        let mut log: Vec<ComplianceAuditEntry> = env
            .storage()
            .persistent()
            .get(&ComplianceKey::AuditLog)
            .unwrap_or_else(|| Vec::new(env));
        log.push_back(ComplianceAuditEntry {
            id,
            actor,
            action,
            detail,
            timestamp: env.ledger().timestamp(),
        });
        env.storage()
            .persistent()
            .set(&ComplianceKey::AuditLog, &log);
    }

    pub fn get_audit_log(env: &Env) -> Vec<ComplianceAuditEntry> {
        env.storage()
            .persistent()
            .get(&ComplianceKey::AuditLog)
            .unwrap_or_else(|| Vec::new(env))
    }
}
