use soroban_sdk::{Address, Env, String, Vec};
use crate::storage_keys::TrustAndSafetyKey;
use crate::types::{CategoryLicenseConfig, FraudRecord, ReviewerIdentity, ReviewerPoints};
use crate::errors::ContractError;
use crate::admin_manager::AdminManager;

pub struct TrustAndSafety;

impl TrustAndSafety {
    pub fn get_fraud_record(env: &Env, project_id: u64) -> FraudRecord {
        env.storage()
            .persistent()
            .get(&TrustAndSafetyKey::ProjectFraudRecord(project_id))
            .unwrap_or(FraudRecord {
                is_flagged: false,
                flag_reason: None,
                rejection_count: 0,
                reversal_count: 0,
            })
    }

    pub fn save_fraud_record(env: &Env, project_id: u64, record: &FraudRecord) {
        env.storage()
            .persistent()
            .set(&TrustAndSafetyKey::ProjectFraudRecord(project_id), record);
    }

    pub fn flag_project_fraud(env: &Env, admin: &Address, project_id: u64, reason: String) -> Result<(), ContractError> {
        if !AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }
        let mut record = Self::get_fraud_record(env, project_id);
        record.is_flagged = true;
        record.flag_reason = Some(reason);
        Self::save_fraud_record(env, project_id, &record);
        Ok(())
    }

    pub fn record_verification_rejection(env: &Env, project_id: u64) {
        let mut record = Self::get_fraud_record(env, project_id);
        record.rejection_count = record.rejection_count.saturating_add(1);
        if record.rejection_count >= 3 {
            record.is_flagged = true;
            record.flag_reason = Some(String::from_str(env, "Multiple verification rejections"));
        }
        Self::save_fraud_record(env, project_id, &record);
    }

    pub fn record_verification_reversal(env: &Env, project_id: u64) {
        let mut record = Self::get_fraud_record(env, project_id);
        record.reversal_count = record.reversal_count.saturating_add(1);
        if record.reversal_count >= 2 {
            record.is_flagged = true;
            record.flag_reason = Some(String::from_str(env, "Multiple verification reversals"));
        }
        Self::save_fraud_record(env, project_id, &record);
    }

    // #789 Licenses
    pub fn set_category_license_config(
        env: &Env,
        admin: &Address,
        category: String,
        approved_licenses: Vec<String>,
        exceptions_allowed: bool,
    ) -> Result<(), ContractError> {
        if !AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }
        let config = CategoryLicenseConfig {
            approved_licenses,
            exceptions_allowed,
        };
        env.storage()
            .persistent()
            .set(&TrustAndSafetyKey::CategoryLicenseConfig(category), &config);
        Ok(())
    }

    pub fn get_category_license_config(env: &Env, category: &String) -> Option<CategoryLicenseConfig> {
        env.storage()
            .persistent()
            .get(&TrustAndSafetyKey::CategoryLicenseConfig(category.clone()))
    }

    pub fn verify_license_for_category(env: &Env, category: &String, license: &Option<String>) -> bool {
        let config = match Self::get_category_license_config(env, category) {
            Some(c) => c,
            None => return true, // If no config, assume all allowed
        };
        if config.exceptions_allowed {
            return true;
        }
        let proj_license = match license {
            Some(l) => l,
            None => return false,
        };
        config.approved_licenses.contains(proj_license)
    }

    // #790 Reviewer Identity
    pub fn verify_reviewer_identity(
        env: &Env,
        admin: &Address,
        reviewer: Address,
        email_verified: bool,
        social_proof_verified: bool,
        verification_method: Option<String>,
    ) -> Result<(), ContractError> {
        if !AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }
        let identity = ReviewerIdentity {
            is_verified: true,
            email_verified,
            social_proof_verified,
            verification_method,
        };
        env.storage()
            .persistent()
            .set(&TrustAndSafetyKey::ReviewerIdentity(reviewer.clone()), &identity);
        Ok(())
    }

    pub fn get_reviewer_identity(env: &Env, reviewer: &Address) -> ReviewerIdentity {
        env.storage()
            .persistent()
            .get(&TrustAndSafetyKey::ReviewerIdentity(reviewer.clone()))
            .unwrap_or(ReviewerIdentity {
                is_verified: false,
                email_verified: false,
                social_proof_verified: false,
                verification_method: None,
            })
    }

    // #791 Rewards
    pub fn award_reviewer_points(env: &Env, reviewer: &Address, points: u64, is_quality_review: bool) {
        let mut rp = Self::get_reviewer_points(env, reviewer);
        rp.total_points = rp.total_points.saturating_add(points);
        if is_quality_review {
            rp.quality_reviews_count = rp.quality_reviews_count.saturating_add(1);
        }
        // Auto-grant badge
        if rp.total_points >= 100 && !rp.badges.contains(&String::from_str(env, "Top Reviewer")) {
            rp.badges.push_back(String::from_str(env, "Top Reviewer"));
        }
        env.storage()
            .persistent()
            .set(&TrustAndSafetyKey::ReviewerPoints(reviewer.clone()), &rp);
    }

    pub fn get_reviewer_points(env: &Env, reviewer: &Address) -> ReviewerPoints {
        env.storage()
            .persistent()
            .get(&TrustAndSafetyKey::ReviewerPoints(reviewer.clone()))
            .unwrap_or(ReviewerPoints {
                total_points: 0,
                quality_reviews_count: 0,
                badges: Vec::new(env),
            })
    }
}
