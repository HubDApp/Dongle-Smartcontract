//! Test suite organized by domain area.

// Existing test modules
mod admin;
mod error_handling_tests;
mod registration;
mod review;
mod verification;

// New test modules
mod authorization;
mod events;
mod pagination;

// Test infrastructure
pub mod fixtures;
