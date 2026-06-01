//! Test suite organized by domain area.

// Existing test modules
mod admin;
mod archival;
mod error_handling_tests;
mod fee;
mod indexer;
mod registration;
mod review;
mod transfer;
mod verification;

// New test modules
mod authorization;
mod events;
mod pagination;

// Test infrastructure
pub mod fixtures;
