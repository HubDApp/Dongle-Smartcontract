//! Verification requests with ownership and fee checks, events, and state machine.

mod assignment;
mod state_machine;
mod storage;
mod validation;

pub use assignment::VerificationAssignmentRegistry;
pub use state_machine::VerificationStateMachine;
pub use storage::VerificationRegistry;
