//! Admin session management with expiry for secure admin operations
//!
//! Implements session tokens with configurable timeout (default 24h),
//! automatic invalidation on inactivity, and session history/audit log.

use crate::errors::ContractError;
use crate::storage_keys::ExtensionKey2;
use crate::storage_manager::StorageManager;
use soroban_sdk::{contracttype, Address, Env, String, Vec};

/// Default session timeout: 24 hours in seconds
pub const DEFAULT_SESSION_TIMEOUT_SECS: u64 = 24 * 60 * 60;

/// Maximum session timeout: 7 days in seconds
pub const MAX_SESSION_TIMEOUT_SECS: u64 = 7 * 24 * 60 * 60;

/// Inactivity timeout: 1 hour in seconds
pub const INACTIVITY_TIMEOUT_SECS: u64 = 60 * 60;

/// Maximum number of active sessions per admin
pub const MAX_SESSIONS_PER_ADMIN: u32 = 5;

/// Maximum session history entries to keep
pub const MAX_SESSION_HISTORY: u32 = 100;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminSession {
    /// Unique session identifier
    pub session_id: u64,
    /// Admin address that owns this session
    pub admin: Address,
    /// Session creation timestamp
    pub created_at: u64,
    /// Session expiry timestamp
    pub expires_at: u64,
    /// Last activity timestamp (for inactivity check)
    pub last_activity_at: u64,
    /// Whether the session is currently active
    pub is_active: bool,
    /// Session description (e.g., "CLI access", "Dashboard")
    pub description: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionAuditEntry {
    /// Session ID
    pub session_id: u64,
    /// Admin address
    pub admin: Address,
    /// Action performed (created, refreshed, revoked, expired)
    pub action: SessionAction,
    /// Timestamp of the action
    pub timestamp: u64,
    /// Optional description
    pub description: String,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionAction {
    Created,
    Refreshed,
    Revoked,
    Expired,
    Invalidated,
}

pub struct AdminSessionManager;

impl AdminSessionManager {
    /// Create a new admin session with configurable timeout
    pub fn create_session(
        env: &Env,
        admin: Address,
        timeout_secs: Option<u64>,
        description: Option<String>,
    ) -> Result<u64, ContractError> {
        let timeout = timeout_secs.unwrap_or(DEFAULT_SESSION_TIMEOUT_SECS);
        if timeout > MAX_SESSION_TIMEOUT_SECS {
            return Err(ContractError::InvalidProjectData);
        }

        let now = env.ledger().timestamp();
        let session_id = Self::next_session_id(env);

        let session = AdminSession {
            session_id,
            admin: admin.clone(),
            created_at: now,
            expires_at: now + timeout,
            last_activity_at: now,
            is_active: true,
            description: description.unwrap_or_else(|| String::from_str(env, "Admin session")),
        };

        // Store the session
        env.storage().persistent().set(
            &ExtensionKey2::AdminSession(session_id),
            &session,
        );

        // Add to admin's session list
        let mut sessions = Self::get_admin_sessions(env, &admin);
        if sessions.len() >= MAX_SESSIONS_PER_ADMIN {
            // Revoke oldest session
            if let Some(oldest) = sessions.get(0) {
                Self::revoke_session_internal(env, oldest.session_id)?;
            }
            sessions = Self::get_admin_sessions(env, &admin);
        }
        sessions.push_back(session_id);
        env.storage().persistent().set(
            &ExtensionKey2::AdminSessionList(admin.clone()),
            &sessions,
        );

        // Record audit entry
        Self::record_audit(
            env,
            session_id,
            &admin,
            SessionAction::Created,
            &description.unwrap_or_else(|| String::from_str(env, "Session created")),
        );

        Ok(session_id)
    }

    /// Validate that a session is active and not expired
    pub fn validate_session(env: &Env, admin: &Address, session_id: u64) -> Result<(), ContractError> {
        let session = Self::get_session(env, session_id)
            .ok_or(ContractError::Unauthorized)?;

        if session.admin != *admin {
            return Err(ContractError::Unauthorized);
        }

        if !session.is_active {
            return Err(ContractError::Unauthorized);
        }

        let now = env.ledger().timestamp();
        if now > session.expires_at {
            // Mark as expired
            Self::expire_session(env, session_id)?;
            return Err(ContractError::Unauthorized);
        }

        // Check inactivity
        if now - session.last_activity_at > INACTIVITY_TIMEOUT_SECS {
            Self::invalidate_session(env, session_id)?;
            return Err(ContractError::Unauthorized);
        }

        Ok(())
    }

    /// Refresh session activity timestamp
    pub fn refresh_session(env: &Env, session_id: u64) -> Result<(), ContractError> {
        let mut session = Self::get_session(env, session_id)
            .ok_or(ContractError::Unauthorized)?;

        if !session.is_active {
            return Err(ContractError::Unauthorized);
        }

        let now = env.ledger().timestamp();
        if now > session.expires_at {
            return Err(ContractError::Unauthorized);
        }

        session.last_activity_at = now;
        env.storage().persistent().set(
            &ExtensionKey2::AdminSession(session_id),
            &session,
        );

        Self::record_audit(
            env,
            session_id,
            &session.admin,
            SessionAction::Refreshed,
            &String::from_str(env, "Session refreshed"),
        );

        Ok(())
    }

    /// Revoke a session
    pub fn revoke_session(env: &Env, admin: &Address, session_id: u64) -> Result<(), ContractError> {
        let session = Self::get_session(env, session_id)
            .ok_or(ContractError::Unauthorized)?;

        if session.admin != *admin {
            return Err(ContractError::Unauthorized);
        }

        Self::revoke_session_internal(env, session_id)
    }

    /// Internal session revocation
    fn revoke_session_internal(env: &Env, session_id: u64) -> Result<(), ContractError> {
        let mut session = Self::get_session(env, session_id)
            .ok_or(ContractError::Unauthorized)?;

        session.is_active = false;
        env.storage().persistent().set(
            &ExtensionKey2::AdminSession(session_id),
            &session,
        );

        Self::record_audit(
            env,
            session_id,
            &session.admin,
            SessionAction::Revoked,
            &String::from_str(env, "Session revoked"),
        );

        Ok(())
    }

    /// Mark session as expired
    fn expire_session(env: &Env, session_id: u64) -> Result<(), ContractError> {
        let mut session = Self::get_session(env, session_id)
            .ok_or(ContractError::Unauthorized)?;

        session.is_active = false;
        env.storage().persistent().set(
            &ExtensionKey2::AdminSession(session_id),
            &session,
        );

        Self::record_audit(
            env,
            session_id,
            &session.admin,
            SessionAction::Expired,
            &String::from_str(env, "Session expired"),
        );

        Ok(())
    }

    /// Mark session as invalid due to inactivity
    fn invalidate_session(env: &Env, session_id: u64) -> Result<(), ContractError> {
        let mut session = Self::get_session(env, session_id)
            .ok_or(ContractError::Unauthorized)?;

        session.is_active = false;
        env.storage().persistent().set(
            &ExtensionKey2::AdminSession(session_id),
            &session,
        );

        Self::record_audit(
            env,
            session_id,
            &session.admin,
            SessionAction::Invalidated,
            &String::from_str(env, "Session invalidated due to inactivity"),
        );

        Ok(())
    }

    /// Get a session by ID
    pub fn get_session(env: &Env, session_id: u64) -> Option<AdminSession> {
        env.storage().persistent().get(&ExtensionKey2::AdminSession(session_id))
    }

    /// Get all sessions for an admin
    pub fn get_admin_sessions(env: &Env, admin: &Address) -> Vec<u64> {
        env.storage().persistent()
            .get(&ExtensionKey2::AdminSessionList(admin.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Get session history for an admin
    pub fn get_session_history(env: &Env, admin: &Address) -> Vec<SessionAuditEntry> {
        env.storage().persistent()
            .get(&ExtensionKey2::AdminSessionHistory(admin.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Get session history with pagination
    pub fn get_session_history_paginated(
        env: &Env,
        admin: &Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<SessionAuditEntry> {
        let history = Self::get_session_history(env, admin);
        let end = (start_index + limit).min(history.len());
        let mut result = Vec::new(env);
        for i in start_index..end {
            if let Some(entry) = history.get(i) {
                result.push_back(entry);
            }
        }
        result
    }

    /// Record audit entry
    fn record_audit(
        env: &Env,
        session_id: u64,
        admin: &Address,
        action: SessionAction,
        description: &String,
    ) {
        let entry = SessionAuditEntry {
            session_id,
            admin: admin.clone(),
            action,
            timestamp: env.ledger().timestamp(),
            description: description.clone(),
        };

        let mut history = Self::get_session_history(env, admin);
        history.push_back(entry);

        // Trim to max history size
        while history.len() > MAX_SESSION_HISTORY {
            history.remove(0);
        }

        env.storage().persistent().set(
            &ExtensionKey2::AdminSessionHistory(admin.clone()),
            &history,
        );
    }

    /// Get next session ID
    fn next_session_id(env: &Env) -> u64 {
        env.storage().persistent()
            .get::<_, u64>(&ExtensionKey2::NextAdminSessionId)
            .unwrap_or(0) + 1
    }

    /// Get all active sessions (admin-only operation)
    pub fn get_all_active_sessions(env: &Env) -> Vec<AdminSession> {
        let admins = crate::admin_manager::AdminManager::get_admin_list(env);
        let mut active_sessions = Vec::new(env);

        for admin in admins.iter() {
            let sessions = Self::get_admin_sessions(env, &admin);
            for session_id in sessions.iter() {
                if let Some(session) = Self::get_session(env, session_id) {
                    if session.is_active {
                        let now = env.ledger().timestamp();
                        if now <= session.expires_at {
                            active_sessions.push_back(session);
                        }
                    }
                }
            }
        }

        active_sessions
    }

    /// Cleanup expired sessions (can be called periodically)
    pub fn cleanup_expired_sessions(env: &Env, batch_size: u32) -> Result<u32, ContractError> {
        let admins = crate::admin_manager::AdminManager::get_admin_list(env);
        let now = env.ledger().timestamp();
        let mut cleaned = 0u32;

        for admin in admins.iter() {
            if cleaned >= batch_size {
                break;
            }

            let sessions = Self::get_admin_sessions(env, &admin);
            for session_id in sessions.iter() {
                if cleaned >= batch_size {
                    break;
                }

                if let Some(session) = Self::get_session(env, session_id) {
                    if session.is_active && now > session.expires_at {
                        Self::expire_session(env, session_id)?;
                        cleaned += 1;
                    }
                }
            }
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_create_session() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let session_id = AdminSessionManager::create_session(
            &env,
            admin.clone(),
            None,
            None,
        ).unwrap();

        let session = AdminSessionManager::get_session(&env, session_id).unwrap();
        assert!(session.is_active);
        assert_eq!(session.admin, admin);
    }

    #[test]
    fn test_validate_session() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let session_id = AdminSessionManager::create_session(
            &env,
            admin.clone(),
            Some(3600), // 1 hour
            None,
        ).unwrap();

        assert!(AdminSessionManager::validate_session(&env, &admin, session_id).is_ok());
    }

    #[test]
    fn test_revoke_session() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let session_id = AdminSessionManager::create_session(
            &env,
            admin.clone(),
            None,
            None,
        ).unwrap();

        AdminSessionManager::revoke_session(&env, &admin, session_id).unwrap();

        let session = AdminSessionManager::get_session(&env, session_id).unwrap();
        assert!(!session.is_active);
    }
}
