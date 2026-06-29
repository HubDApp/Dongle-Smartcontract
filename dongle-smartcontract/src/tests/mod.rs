//! Test suite organized by domain area.

// Existing test modules
mod admin;
mod admin_action_log;
mod archival;
mod collections;
mod error_handling_tests;
mod featured;
// mod fee;
// mod indexer;
mod review;

// New test modules
// mod authorization;
// mod basic_new_features;
mod cleanup;
mod events;
mod moderation;
// mod pagination;
mod claim;
mod dependencies;
mod maintainers;
mod renewal;
mod review_settings;
mod security_contact;
mod verification_features;
mod verification;

// String validation: names, descriptions, CIDs, categories, URLs
mod string_validation;
mod license_metadata;

// Metadata freeze policy for verified projects
// mod verified_freeze;

// Fee token rotation and payment behavior
mod fee_token_rotation;

// Storage field size boundary tests
mod field_limits;

// Storage index size limits (owner projects, reviews)
// mod index_limits;

// Security invariant tests: stats, owner index, verification, admin count
mod invariants;

// Property-based pagination tests using proptest
// mod proptest_pagination;
mod proptest_pagination;
// Issue #221: fee amount boundary tests
mod fee_boundary;
// Issues #240, #241, #246: review tombstones, sorting, cooldown
mod review_features;

// Test infrastructure
mod bookmarks;
mod duplicate_dispute;
mod endorsements;
pub mod fixtures;
mod issues_242_252_256;
mod linked_projects;
mod multisig_and_history;
mod subscriptions;
mod timelock;

// Atomicity tests for multi-storage operations
// mod atomicity;

// Project region metadata (#238) and integrity hash (#250)
mod region_and_integrity;
