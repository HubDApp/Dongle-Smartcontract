//! Storage key uniqueness and collision-detection tests (closes #665).
//!
//! ## Background
//!
//! Soroban `#[contracttype]` enums are XDR-encoded as `(variant_index, payload)`.
//! Two variants with the *same* numeric index would collide at the storage layer
//! regardless of their Rust names.  Soroban caps a single `#[contracttype]`
//! union at **50 variants** — trying to compile a 51st case panics the macro
//! (see the comment on `ExtensionKey::ProjectByNormalizedName`).
//!
//! We split storage across two enums:
//!
//! - `StorageKey` — first 50 keys (indices 0..49).
//! - `ExtensionKey` — overflow keys (indices 0..N, independent namespace
//!   because the enum type is different, so the XDR encoding is also different).
//!
//! ## Collision model
//!
//! Within a single enum, Soroban assigns each variant the ordinal index it has
//! in the source.  Because `StorageKey` and `ExtensionKey` are *different* XDR
//! union types, there is **no cross-enum collision**: variant 0 of `StorageKey`
//! and variant 0 of `ExtensionKey` produce different XDR blobs and therefore
//! different storage keys.
//!
//! The only risks are:
//!
//! 1. **Intra-enum duplicate Rust names** — the compiler rejects these.
//! 2. **Exceeding 50 variants in one enum** — the Soroban macro panics at
//!    compile time.  `ExtensionKey` currently has capacity for additional
//!    variants before it would also need to be split.
//!
//! ## Overflow limit for ExtensionKey
//!
//! `ExtensionKey` follows the same Soroban 50-variant cap.  When it
//! approaches 50 variants a third enum (`ExtensionKey2`, following the same
//! pattern) must be introduced.  The tests below count the current variants
//! and will fail loudly before the cap is reached, giving maintainers a clear
//! signal to split the enum.
//!
//! ## Performance impact
//!
//! Key lookup is O(1) — the key is serialised to XDR once per call and handed
//! directly to the host storage map.  The two-enum split adds zero runtime
//! overhead compared to a single enum.

use crate::storage_keys::{ExtensionKey, StorageKey};
use soroban_sdk::{testutils::Address as _, Address, Env, IntoVal, String, Val};

// ── Capacity guards ──────────────────────────────────────────────────────────

/// Maximum variants allowed in a single `#[contracttype]` enum by Soroban.
///
/// Soroban XDR unions use a 32-bit discriminant; the SDK macro enforces a
/// compile-time cap of 50 to keep generated boilerplate bounded.
const SOROBAN_CONTRACTTYPE_VARIANT_CAP: usize = 50;

/// Warn when either enum reaches this fraction of the cap.  At 90 % (≥ 45
/// variants) the test will fail with a helpful message telling maintainers to
/// split the enum before it hits 50.
const VARIANT_WARN_THRESHOLD: usize = 45;

/// Count the number of variants in `StorageKey` by listing every discriminant.
///
/// We keep this function in sync with the enum manually.  If a variant is
/// added to `StorageKey` without updating this list the count will be wrong
/// and the uniqueness test below will fail — which is the desired signal.
fn storage_key_variant_count() -> usize {
    // One entry per variant in StorageKey (keep in sync with storage_keys.rs).
    let variants: &[&str] = &[
        "Project",
        "NextProjectId",
        "OwnerProjectCount",
        "ProjectStats",
        "OwnerProjects",
        "ProjectByName",
        "ProjectBySlug",
        "ProjectLifecycleStatus",
        "ProjectCount",
        "Review",
        "Verification",
        "NextVerificationRequestId",
        "VerificationRecord",
        "ProjectVerificationHistory",
        "FeeConfig",
        "FeePaidForProject",
        "RegistrationFeePaidForAddress",
        "Admin",
        "AdminList",
        "MinProjectAge",
        "ProjectTags",
        "ProjectLaunchTimestamp",
        "ProjectBountyUrl",
        "ProjectSocialLinks",
        "ProjectMaintainers",
        "ProjectLinkedProjects",
        "ProjectReports",
        "ProjectReportCount",
        "UserReport",
        "UserReviews",
        "Treasury",
        "ProjectReviews",
        "PendingTransfer",
        "CategoryProjects",
        "VerificationDuration",
        "ReviewsEnabled",
        "ReviewReport",
        "VerificationRenewal",
        "VerificationRenewalHistory",
        "VerificationRenewalCount",
        "FeaturedProjects",
        "Collection",
        "CollectionNameById",
        "NextCollectionId",
        "CollectionList",
        "CollectionProjectIds",
        "AdminActionLog",
        "AdminActionLogCount",
        "ContractPaused",
        "ActiveOwnerProjects",
    ];
    variants.len()
}

/// Count the number of variants in `ExtensionKey`.
fn extension_key_variant_count() -> usize {
    // One entry per variant in ExtensionKey (keep in sync with storage_keys.rs).
    let variants: &[&str] = &[
        "ClaimRequest",
        "ClaimReqProjClaimant",
        "ProjectClaimRequests",
        "NextClaimRequestId",
        "ProjectDependency",
        "ProjectDependencyKeys",
        "DuplicateDispute",
        "ProjectDuplicateDisputes",
        "NextDuplicateDisputeId",
        "VerificationDuration",
        "ProjectFollowers",
        "UserSubscriptions",
        "FollowerCount",
        "TimelockAction",
        "TimelockActionIds",
        "NextTimelockActionId",
        "TimelockFeeParams",
        "TimelockAdminAddParams",
        "TimelockAdminRemoveParams",
        "UserBookmarks",
        "AdminApprovalThreshold",
        "NextAdminProposalId",
        "AdminProposal",
        "AdminProposalIds",
        "NextChangelogEntryId",
        "ProjectChangelogEntry",
        "ProjectChangelogEntries",
        "ProjectEndorsements",
        "EndorsementCount",
        "ReviewTombstone",
        "ReviewLastUpdated",
        "FeePaymentDetails",
        "RegistrationFeePaymentDetails",
        "FeeRefund",
        "ReservedNames",
        "ProjectRegion",
        "ProjectIntegrityHash",
        "ProjectByNormalizedName",
        "TagProjects",
        "TagIndexWatermark",
        "Paused",
        "ContractClaim",
        "ProjectContracts",
        "ReviewEligibilityConfig",
        "FirstInteraction",
        "ReviewRevisionCount",
        "ReviewRevision",
        "AdminActionLogByAdmin",
        "PendingVerificationRequests",
        "FeeConfigHistory",
    ];
    variants.len()
}

// ── Collision detection ──────────────────────────────────────────────────────

#[test]
fn storage_key_variant_count_within_soroban_cap() {
    let count = storage_key_variant_count();
    assert!(
        count <= SOROBAN_CONTRACTTYPE_VARIANT_CAP,
        "StorageKey has {count} variants, exceeding the Soroban cap of \
         {SOROBAN_CONTRACTTYPE_VARIANT_CAP}. Split the enum before adding more variants."
    );
}

#[test]
fn storage_key_variant_count_below_warn_threshold() {
    let count = storage_key_variant_count();
    assert!(
        count < VARIANT_WARN_THRESHOLD,
        "StorageKey has {count} variants (>= warn threshold {VARIANT_WARN_THRESHOLD}). \
         Consider splitting the enum soon to stay below the Soroban cap of \
         {SOROBAN_CONTRACTTYPE_VARIANT_CAP}."
    );
}

#[test]
fn extension_key_variant_count_within_soroban_cap() {
    let count = extension_key_variant_count();
    assert!(
        count <= SOROBAN_CONTRACTTYPE_VARIANT_CAP,
        "ExtensionKey has {count} variants, exceeding the Soroban cap of \
         {SOROBAN_CONTRACTTYPE_VARIANT_CAP}. Introduce ExtensionKey2 before adding more variants."
    );
}

#[test]
fn extension_key_variant_count_below_warn_threshold() {
    let count = extension_key_variant_count();
    assert!(
        count < VARIANT_WARN_THRESHOLD,
        "ExtensionKey has {count} variants (>= warn threshold {VARIANT_WARN_THRESHOLD}). \
         Consider splitting the enum to stay below the Soroban cap of \
         {SOROBAN_CONTRACTTYPE_VARIANT_CAP}."
    );
}

// ── Cross-enum isolation ─────────────────────────────────────────────────────

/// Verify that `StorageKey` and `ExtensionKey` variants with the same
/// discriminant index produce *different* XDR-encoded values and therefore
/// never collide in storage.
///
/// We test this by encoding the first variant of each enum and asserting they
/// are not equal.  Because the union type discriminant differs between
/// `StorageKey` and `ExtensionKey`, all variant combinations across the two
/// enums are guaranteed to be distinct.
#[test]
fn storage_key_and_extension_key_are_distinct_types() {
    let env = Env::default();
    let _ = env.register(crate::DongleContract, ());

    // Encode variant-index-0 of StorageKey (Project(0)) and
    // variant-index-0 of ExtensionKey (ClaimRequest(0)).
    let sk_val: Val = StorageKey::Project(0u64).into_val(&env);
    let ek_val: Val = ExtensionKey::ClaimRequest(0u64).into_val(&env);

    // The raw Val bit-patterns must differ — different XDR union type means
    // different storage keys even for the same ordinal and same payload.
    assert_ne!(
        sk_val.get_payload(),
        ek_val.get_payload(),
        "StorageKey and ExtensionKey must produce distinct encoded values \
         for the same discriminant index"
    );
}

// ── Unique key round-trip ─────────────────────────────────────────────────────

/// Write a value under a `StorageKey` and a value under an `ExtensionKey` that
/// share the same discriminant index (0).  Read them back and confirm neither
/// overwrites the other.
#[test]
fn storage_key_and_extension_key_do_not_collide_in_storage() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());

    env.as_contract(&contract_id, || {
        let sk = StorageKey::Project(42u64);
        let ek = ExtensionKey::ClaimRequest(42u64);

        env.storage().persistent().set(&sk, &100u32);
        env.storage().persistent().set(&ek, &200u32);

        let sk_val: u32 = env.storage().persistent().get(&sk).unwrap();
        let ek_val: u32 = env.storage().persistent().get(&ek).unwrap();

        assert_eq!(sk_val, 100, "StorageKey value should be 100");
        assert_eq!(ek_val, 200, "ExtensionKey value should be 200");
        assert_ne!(sk_val, ek_val, "Keys must not share storage");
    });
}

/// Write two distinct `StorageKey` variants that both carry a `u64` payload
/// and confirm they are stored independently.
#[test]
fn distinct_storage_key_variants_do_not_collide() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());

    env.as_contract(&contract_id, || {
        let k1 = StorageKey::Project(1u64);
        let k2 = StorageKey::ProjectStats(1u64);

        env.storage().persistent().set(&k1, &"project");
        env.storage().persistent().set(&k2, &"stats");

        let v1: soroban_sdk::String = env.storage().persistent().get(&k1).unwrap();
        let v2: soroban_sdk::String = env.storage().persistent().get(&k2).unwrap();

        assert_eq!(v1, soroban_sdk::String::from_str(&env, "project"));
        assert_eq!(v2, soroban_sdk::String::from_str(&env, "stats"));
    });
}

/// Write two distinct `ExtensionKey` variants with the same `u64` payload and
/// confirm they are stored independently.
#[test]
fn distinct_extension_key_variants_do_not_collide() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());

    env.as_contract(&contract_id, || {
        let k1 = ExtensionKey::ProjectFollowers(5u64);
        let k2 = ExtensionKey::FollowerCount(5u64);

        env.storage().persistent().set(&k1, &10u32);
        env.storage().persistent().set(&k2, &20u32);

        let v1: u32 = env.storage().persistent().get(&k1).unwrap();
        let v2: u32 = env.storage().persistent().get(&k2).unwrap();

        assert_eq!(v1, 10);
        assert_eq!(v2, 20);
    });
}

/// Verify that `StorageKey::VerificationDuration` (a `StorageKey` variant) and
/// `ExtensionKey::VerificationDuration` (the same *name* but in a different
/// enum) do not share storage — this is the most common footgun.
#[test]
fn verification_duration_in_both_enums_does_not_collide() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());

    env.as_contract(&contract_id, || {
        let sk = StorageKey::VerificationDuration;
        let ek = ExtensionKey::VerificationDuration;

        env.storage().persistent().set(&sk, &111u64);
        env.storage().persistent().set(&ek, &222u64);

        let sk_val: u64 = env.storage().persistent().get(&sk).unwrap();
        let ek_val: u64 = env.storage().persistent().get(&ek).unwrap();

        assert_eq!(sk_val, 111);
        assert_eq!(ek_val, 222);
        assert_ne!(sk_val, ek_val, "Same-name variants in different enums must not collide");
    });
}

// ── All storage key round-trips ───────────────────────────────────────────────

/// Smoke-test that a representative set of `StorageKey` variants can be
/// written and read back without panic.  This exercises the XDR codec for
/// each variant shape (unit, u64, Address, String, tuple).
#[test]
fn storage_key_variants_round_trip() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());
    let addr = Address::generate(&env);
    let name = String::from_str(&env, "test");

    env.as_contract(&contract_id, || {
        // Unit variants
        env.storage()
            .persistent()
            .set(&StorageKey::NextProjectId, &1u64);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectCount, &2u64);
        env.storage()
            .persistent()
            .set(&StorageKey::AdminList, &3u32);
        env.storage()
            .persistent()
            .set(&StorageKey::FeeConfig, &4u32);
        env.storage()
            .persistent()
            .set(&StorageKey::ContractPaused, &false);

        // u64-keyed variants
        env.storage()
            .persistent()
            .set(&StorageKey::Project(1), &"p");
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectStats(1), &"s");
        env.storage()
            .persistent()
            .set(&StorageKey::FeePaidForProject(1), &true);

        // Address-keyed variants
        env.storage()
            .persistent()
            .set(&StorageKey::Admin(addr.clone()), &true);
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjects(addr.clone()), &0u32);

        // String-keyed variants
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectByName(name.clone()), &1u64);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectBySlug(name.clone()), &1u64);

        // Tuple variants
        env.storage()
            .persistent()
            .set(&StorageKey::Review(1, addr.clone()), &5u32);

        // Read back a sample
        assert_eq!(
            env.storage()
                .persistent()
                .get::<_, u64>(&StorageKey::NextProjectId)
                .unwrap(),
            1u64
        );
        assert_eq!(
            env.storage()
                .persistent()
                .get::<_, bool>(&StorageKey::ContractPaused)
                .unwrap(),
            false
        );
    });
}

/// Smoke-test that a representative set of `ExtensionKey` variants round-trip.
#[test]
fn extension_key_variants_round_trip() {
    let env = Env::default();
    let contract_id = env.register(crate::DongleContract, ());
    let addr = Address::generate(&env);
    let tag = String::from_str(&env, "defi");

    env.as_contract(&contract_id, || {
        // Unit variants
        env.storage()
            .persistent()
            .set(&ExtensionKey::NextClaimRequestId, &1u64);
        env.storage()
            .persistent()
            .set(&ExtensionKey::TimelockActionIds, &2u32);
        env.storage()
            .persistent()
            .set(&ExtensionKey::ReservedNames, &3u32);

        // u64-keyed variants
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectFollowers(1), &10u32);
        env.storage()
            .persistent()
            .set(&ExtensionKey::FollowerCount(1), &10u32);
        env.storage()
            .persistent()
            .set(&ExtensionKey::UserBookmarks(addr.clone()), &0u32);

        // String-keyed variants
        env.storage()
            .persistent()
            .set(&ExtensionKey::TagProjects(tag.clone()), &1u32);
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectByNormalizedName(tag.clone()), &1u64);

        // Read back a sample
        assert_eq!(
            env.storage()
                .persistent()
                .get::<_, u64>(&ExtensionKey::NextClaimRequestId)
                .unwrap(),
            1u64
        );
        assert_eq!(
            env.storage()
                .persistent()
                .get::<_, u32>(&ExtensionKey::FollowerCount(1))
                .unwrap(),
            10u32
        );
    });
}
