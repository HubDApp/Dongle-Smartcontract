use crate::constants::{
    LEDGER_BUMP_CRITICAL, LEDGER_THRESHOLD_CRITICAL, PROJECT_QUERIES_PER_MINUTE,
    PROJECT_REGISTRATIONS_PER_DAY, PROJECT_UPDATES_PER_DAY,
    VERIFIED_PROJECT_OPERATION_LIMIT_MULTIPLIER,
};
use crate::errors::ContractError;
use crate::storage_keys::{ProjectRateLimitKey, StorageKey};
use crate::types::{Project, VerificationStatus};
use soroban_sdk::{contracttype, Address, Env, Vec};

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;
const SECONDS_PER_MINUTE: u64 = 60;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
struct OperationWindow {
    bucket: u64,
    count: u32,
}

#[derive(Clone, Copy)]
enum Operation {
    Registration,
    Update,
    Query,
}

pub struct ProjectOperationLimiter;

impl ProjectOperationLimiter {
    pub fn consume_registration(env: &Env, address: &Address) -> Result<(), ContractError> {
        Self::consume(
            env,
            address,
            Operation::Registration,
            SECONDS_PER_DAY,
            PROJECT_REGISTRATIONS_PER_DAY,
        )
    }

    pub fn consume_update(env: &Env, address: &Address) -> Result<(), ContractError> {
        Self::consume(
            env,
            address,
            Operation::Update,
            SECONDS_PER_DAY,
            PROJECT_UPDATES_PER_DAY,
        )
    }

    pub fn consume_query(env: &Env, address: &Address) -> Result<(), ContractError> {
        Self::consume(
            env,
            address,
            Operation::Query,
            SECONDS_PER_MINUTE,
            PROJECT_QUERIES_PER_MINUTE,
        )
    }

    fn consume(
        env: &Env,
        address: &Address,
        operation: Operation,
        window_seconds: u64,
        default_limit: u32,
    ) -> Result<(), ContractError> {
        let key = match operation {
            Operation::Registration => ProjectRateLimitKey::Registration(address.clone()),
            Operation::Update => ProjectRateLimitKey::Update(address.clone()),
            Operation::Query => ProjectRateLimitKey::Query(address.clone()),
        };
        let bucket = env.ledger().timestamp() / window_seconds;
        let previous: Option<OperationWindow> = env.storage().persistent().get(&key);
        let previous_count = previous
            .filter(|window| window.bucket == bucket)
            .map(|window| window.count)
            .unwrap_or(0);
        let limit = if Self::is_verified_owner(env, address) {
            default_limit.saturating_mul(VERIFIED_PROJECT_OPERATION_LIMIT_MULTIPLIER)
        } else {
            default_limit
        };

        if previous_count >= limit {
            return Err(ContractError::OperationLimitExceeded);
        }

        env.storage().persistent().set(
            &key,
            &OperationWindow {
                bucket,
                count: previous_count + 1,
            },
        );
        env.storage().persistent().extend_ttl(
            &key,
            LEDGER_THRESHOLD_CRITICAL,
            LEDGER_BUMP_CRITICAL,
        );
        Ok(())
    }

    fn is_verified_owner(env: &Env, address: &Address) -> bool {
        let project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(address.clone()))
            .unwrap_or_else(|| Vec::new(env));

        project_ids.iter().any(|project_id| {
            env.storage()
                .persistent()
                .get::<_, Project>(&StorageKey::Project(project_id))
                .map(|project| {
                    !project.archived
                        && project.verification_status == VerificationStatus::Verified
                })
                .unwrap_or(false)
        })
    }
}