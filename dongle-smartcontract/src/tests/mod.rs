//! Test suite organized by domain area.

// Existing test modules
mod admin;
mod archival;
mod error_handling_tests;
mod fee;
mod featured;
mod indexer;
mod review;

// New test modules
mod authorization;
mod basic_new_features;
mod events;
mod moderation;
mod pagination;
mod review_settings;

// Test infrastructure
pub mod fixtures;
