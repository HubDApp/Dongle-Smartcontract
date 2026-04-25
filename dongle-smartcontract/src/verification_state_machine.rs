//! Verification state machine with strict transition enforcement

use crate::errors::ContractError;
use crate::types::VerificationStatus;

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
            
            // Same state (no change) - this is allowed for idempotency
            (current, target) if current == target => Ok(()),
            
            // All other transitions are invalid
            (from, to) => Err(ContractError::InvalidStatusTransition),
        }
    }
    
    /// Gets a descriptive error message for invalid transitions
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
        matches!(status, VerificationStatus::Unverified | VerificationStatus::Rejected)
    }
    
    /// Checks if a project can be approved based on its current status
    pub fn can_be_approved(status: VerificationStatus) -> bool {
        matches!(status, VerificationStatus::Pending)
    }
    
    /// Checks if a project can be rejected based on its current status
    pub fn can_be_rejected(status: VerificationStatus) -> bool {
        matches!(status, VerificationStatus::Pending)
    }
    
    /// Gets all possible next states from the current state
    pub fn get_possible_next_states(status: VerificationStatus) -> Vec<VerificationStatus> {
        match status {
            VerificationStatus::Unverified => vec![VerificationStatus::Pending],
            VerificationStatus::Pending => vec![
                VerificationStatus::Verified,
                VerificationStatus::Rejected,
            ],
            VerificationStatus::Rejected => vec![VerificationStatus::Pending],
            VerificationStatus::Verified => vec![], // Terminal state
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
        ).is_ok());
        
        // Rejected -> Pending
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Rejected,
            VerificationStatus::Pending
        ).is_ok());
        
        // Pending -> Verified
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Pending,
            VerificationStatus::Verified
        ).is_ok());
        
        // Pending -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Pending,
            VerificationStatus::Rejected
        ).is_ok());
        
        // Same state transitions
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Unverified
        ).is_ok());
    }
    
    #[test]
    fn test_invalid_transitions() {
        // Unverified -> Verified
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Verified
        ).is_err());
        
        // Unverified -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Rejected
        ).is_err());
        
        // Verified -> Pending
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Verified,
            VerificationStatus::Pending
        ).is_err());
        
        // Verified -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Verified,
            VerificationStatus::Rejected
        ).is_err());
    }
    
    #[test]
    fn test_can_request_verification() {
        assert!(VerificationStateMachine::can_request_verification(VerificationStatus::Unverified));
        assert!(VerificationStateMachine::can_request_verification(VerificationStatus::Rejected));
        assert!(!VerificationStateMachine::can_request_verification(VerificationStatus::Pending));
        assert!(!VerificationStateMachine::can_request_verification(VerificationStatus::Verified));
    }
    
    #[test]
    fn test_can_be_approved() {
        assert!(VerificationStateMachine::can_be_approved(VerificationStatus::Pending));
        assert!(!VerificationStateMachine::can_be_approved(VerificationStatus::Unverified));
        assert!(!VerificationStateMachine::can_be_approved(VerificationStatus::Rejected));
        assert!(!VerificationStateMachine::can_be_approved(VerificationStatus::Verified));
    }
    
    #[test]
    fn test_can_be_rejected() {
        assert!(VerificationStateMachine::can_be_rejected(VerificationStatus::Pending));
        assert!(!VerificationStateMachine::can_be_rejected(VerificationStatus::Unverified));
        assert!(!VerificationStateMachine::can_be_rejected(VerificationStatus::Rejected));
        assert!(!VerificationStateMachine::can_be_rejected(VerificationStatus::Verified));
    }
    
    #[test]
    fn test_get_possible_next_states() {
        assert_eq!(
            VerificationStateMachine::get_possible_next_states(VerificationStatus::Unverified),
            vec![VerificationStatus::Pending]
        );
        
        assert_eq!(
            VerificationStateMachine::get_possible_next_states(VerificationStatus::Pending),
            vec![VerificationStatus::Verified, VerificationStatus::Rejected]
        );
        
        assert_eq!(
            VerificationStateMachine::get_possible_next_states(VerificationStatus::Rejected),
            vec![VerificationStatus::Pending]
        );
        
        assert_eq!(
            VerificationStateMachine::get_possible_next_states(VerificationStatus::Verified),
            vec![]
        );
    }
}
