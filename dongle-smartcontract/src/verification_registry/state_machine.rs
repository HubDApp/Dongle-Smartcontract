//! Verification status state machine and transition validation.

use crate::errors::ContractError;
use crate::types::VerificationStatus;
use soroban_sdk::{Env, Vec};

/// Centralized verification state machine
pub struct VerificationStateMachine;

impl VerificationStateMachine {
    /// Validates if a state transition is allowed
    ///
    /// # Arguments
    /// * `current_status` - The current verification status
    /// * `target_status` - The desired verification status
    ///
    /// # Returns
    /// * `Ok(())` if the transition is valid
    /// * `Err(ContractError)` if the transition is invalid
    pub fn validate_transition(
        current_status: VerificationStatus,
        target_status: VerificationStatus,
    ) -> Result<(), ContractError> {
        match (current_status, target_status) {
            // Unverified -> Pending (verification request)
            (VerificationStatus::Unverified, VerificationStatus::Pending) => Ok(()),

            // Rejected -> Pending (re-request verification after rejection)
            (VerificationStatus::Rejected, VerificationStatus::Pending) => Ok(()),

            // Pending -> Verified (admin approval)
            (VerificationStatus::Pending, VerificationStatus::Verified) => Ok(()),

            // Pending -> Rejected (admin rejection)
            (VerificationStatus::Pending, VerificationStatus::Rejected) => Ok(()),

            // Verified -> Unverified (admin revocation)
            (VerificationStatus::Verified, VerificationStatus::Unverified) => Ok(()),

            // Same state (no change) - this should fail as it's not a valid transition
            (current, target) if current == target => Err(ContractError::InvalidStatus),

            // All other transitions are invalid
            (_from, _to) => Err(ContractError::InvalidStatus),
        }
    }

    /// Gets a descriptive error message for invalid transitions
    #[allow(dead_code)]
    fn get_transition_error_message(
        from: VerificationStatus,
        to: VerificationStatus,
    ) -> &'static str {
        match (from, to) {
            (VerificationStatus::Unverified, VerificationStatus::Verified) => {
                "Cannot verify directly from Unverified status. Must request verification first."
            }
            (VerificationStatus::Unverified, VerificationStatus::Rejected) => {
                "Cannot reject from Unverified status. Must request verification first."
            }
            (VerificationStatus::Pending, VerificationStatus::Unverified) => {
                "Cannot return to Unverified from Pending status."
            }
            (VerificationStatus::Verified, VerificationStatus::Pending) => {
                "Cannot request verification for already verified project."
            }
            (VerificationStatus::Verified, VerificationStatus::Rejected) => {
                "Cannot reject already verified project."
            }
            (VerificationStatus::Verified, VerificationStatus::Unverified) => {
                "Cannot unverify already verified project."
            }
            (VerificationStatus::Rejected, VerificationStatus::Verified) => {
                "Cannot verify directly from Rejected status. Must request verification again."
            }
            (VerificationStatus::Rejected, VerificationStatus::Unverified) => {
                "Cannot return to Unverified from Rejected status."
            }
            _ => "Invalid verification status transition.",
        }
    }

    /// Checks if a project can request verification based on its current status
    pub fn can_request_verification(status: VerificationStatus) -> bool {
        matches!(
            status,
            VerificationStatus::Unverified | VerificationStatus::Rejected
        )
    }

    /// Checks if a project can be approved based on its current status
    #[allow(dead_code)]
    pub fn can_be_approved(status: VerificationStatus) -> bool {
        matches!(status, VerificationStatus::Pending)
    }

    /// Checks if a project can be rejected based on its current status
    #[allow(dead_code)]
    pub fn can_be_rejected(status: VerificationStatus) -> bool {
        matches!(status, VerificationStatus::Pending)
    }

    /// Gets all possible next states from the current state
    #[allow(dead_code)]
    pub fn get_possible_next_states(
        env: &Env,
        status: VerificationStatus,
    ) -> Vec<VerificationStatus> {
        match status {
            VerificationStatus::Unverified => {
                let mut v = Vec::new(env);
                v.push_back(VerificationStatus::Pending);
                v
            }
            VerificationStatus::Pending => {
                let mut v = Vec::new(env);
                v.push_back(VerificationStatus::Verified);
                v.push_back(VerificationStatus::Rejected);
                v
            }
            VerificationStatus::Rejected => {
                let mut v = Vec::new(env);
                v.push_back(VerificationStatus::Pending);
                v
            }
            VerificationStatus::Verified => {
                let mut v = Vec::new(env);
                v.push_back(VerificationStatus::Unverified); // revocable by admin
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        // Unverified -> Pending
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Pending
        )
        .is_ok());

        // Rejected -> Pending
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Rejected,
            VerificationStatus::Pending
        )
        .is_ok());

        // Pending -> Verified
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Pending,
            VerificationStatus::Verified
        )
        .is_ok());

        // Pending -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Pending,
            VerificationStatus::Rejected
        )
        .is_ok());
    }

    #[test]
    fn test_invalid_transitions() {
        // Unverified -> Verified
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Verified
        )
        .is_err());

        // Unverified -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Rejected
        )
        .is_err());

        // Verified -> Pending
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Verified,
            VerificationStatus::Pending
        )
        .is_err());

        // Verified -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Verified,
            VerificationStatus::Rejected
        )
        .is_err());
    }

    #[test]
    fn test_can_request_verification() {
        assert!(VerificationStateMachine::can_request_verification(
            VerificationStatus::Unverified
        ));
        assert!(VerificationStateMachine::can_request_verification(
            VerificationStatus::Rejected
        ));
        assert!(!VerificationStateMachine::can_request_verification(
            VerificationStatus::Pending
        ));
        assert!(!VerificationStateMachine::can_request_verification(
            VerificationStatus::Verified
        ));
    }

    #[test]
    fn test_can_be_approved() {
        assert!(VerificationStateMachine::can_be_approved(
            VerificationStatus::Pending
        ));
        assert!(!VerificationStateMachine::can_be_approved(
            VerificationStatus::Unverified
        ));
        assert!(!VerificationStateMachine::can_be_approved(
            VerificationStatus::Rejected
        ));
        assert!(!VerificationStateMachine::can_be_approved(
            VerificationStatus::Verified
        ));
    }

    #[test]
    fn test_can_be_rejected() {
        assert!(VerificationStateMachine::can_be_rejected(
            VerificationStatus::Pending
        ));
        assert!(!VerificationStateMachine::can_be_rejected(
            VerificationStatus::Unverified
        ));
        assert!(!VerificationStateMachine::can_be_rejected(
            VerificationStatus::Rejected
        ));
        assert!(!VerificationStateMachine::can_be_rejected(
            VerificationStatus::Verified
        ));
    }

    #[test]
    fn test_get_possible_next_states() {
        let env = Env::default();

        let unverified_states = VerificationStateMachine::get_possible_next_states(
            &env,
            VerificationStatus::Unverified,
        );
        assert_eq!(unverified_states.len(), 1);
        assert_eq!(
            unverified_states.get(0).unwrap(),
            VerificationStatus::Pending
        );

        let pending_states =
            VerificationStateMachine::get_possible_next_states(&env, VerificationStatus::Pending);
        assert_eq!(pending_states.len(), 2);
        assert_eq!(pending_states.get(0).unwrap(), VerificationStatus::Verified);
        assert_eq!(pending_states.get(1).unwrap(), VerificationStatus::Rejected);

        let rejected_states =
            VerificationStateMachine::get_possible_next_states(&env, VerificationStatus::Rejected);
        assert_eq!(rejected_states.len(), 1);
        assert_eq!(rejected_states.get(0).unwrap(), VerificationStatus::Pending);

        let verified_states =
            VerificationStateMachine::get_possible_next_states(&env, VerificationStatus::Verified);
        assert_eq!(verified_states.len(), 1);
        assert_eq!(
            verified_states.get(0).unwrap(),
            VerificationStatus::Unverified
        );
    }
}
