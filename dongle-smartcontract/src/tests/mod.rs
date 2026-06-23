//! Test suite organized by domain area.

// Existing test modules
mod admin;
mod admin_action_log;
mod archival;
mod collections;
mod error_handling_tests;
mod featured;
mod fee;
mod indexer;
mod review;

// New test modules
mod authorization;
mod basic_new_features;
mod cleanup;
mod events;
mod moderation;
mod pagination;
mod review_settings;
mod dependencies;

// Test infrastructure
pub mod fixtures;

