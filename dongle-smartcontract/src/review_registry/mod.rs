//! Review registry: create/update/delete reviews and maintain aggregates and indexes.

mod storage;
mod validation;

pub use storage::ReviewRegistry;
pub use validation::ReviewValidation;
