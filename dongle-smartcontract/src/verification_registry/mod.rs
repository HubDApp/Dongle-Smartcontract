//! Verification requests with ownership and fee checks, events, and state machine.

mod state_machine;
mod storage;
mod validation;

pub use state_machine::VerificationStateMachine;
pub use storage::VerificationRegistry;
pub use validation::VerificationValidation;
