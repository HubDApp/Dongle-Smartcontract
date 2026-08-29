//! Test suite organized by domain area.

// Existing test modules
mod admin;
mod admin_action_log;
mod archival;
mod collection_registry_crud;
mod collections;
mod error_handling_tests;
mod featured;
// mod fee;
// mod indexer;
mod review;
mod transfer;

// New test modules
// mod authorization;
// mod basic_new_features;
mod cleanup;
mod events;
mod moderation;
// mod pagination;
mod claim;
mod config;
mod dependencies;
mod lifecycle_status;
mod maintainers;
mod renewal;
mod review_history;
mod review_settings;
mod security_contact;
mod verification;
mod verification_features;
mod verification_lifecycle;
mod verification_replacement;

// String validation: names, descriptions, CIDs, categories, URLs
// Issue #545: property-based fuzz tests for the CID and URL validators
mod fuzz_validation;
mod license_metadata;
mod sorted_listing;
mod string_validation;
mod tag_index;
mod tags;

// Metadata freeze policy for verified projects
// mod verified_freeze;

// Fee token rotation and payment behavior
mod fee_token_rotation;

// Full verification-fee payment lifecycle integration test
mod fee_lifecycle;

// Issue #617: multi-step end-to-end workflow integration suite
mod integration_workflows;

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
mod bookmark_pagination;
mod bookmarks;
mod changelog;
mod duplicate_dispute;
mod endorsements;
mod fee_refund;
pub mod fixtures;
mod issues_242_252_256;
mod linked_projects;
// Issues #458, #463, #465, #466: typed-error and admin-log regressions
mod typed_error_regressions;
mod multisig_and_history;
mod proposal_threshold;
mod report_registry;
mod subscriptions;
mod timelock;
mod ttl_batch;

// Atomicity tests for multi-storage operations
// mod atomicity;

// Project region metadata (#238) and integrity hash (#250)
mod region_and_integrity;

// Contract API compatibility (issue #257)
mod api_compat;

// Issue #620: Project metadata validation gaps (https-only, CID charset)
mod validation_620;

// Issue #622: Fee payment state machine enforcement
mod fee_state_machine_622;

// Issue #623: Admin threshold downgrade supermajority edge cases
mod supermajority_623;

// Issue #694: integer overflow/underflow audit
mod arithmetic_overflow_694;
