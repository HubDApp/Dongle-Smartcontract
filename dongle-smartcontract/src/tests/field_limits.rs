//! Field size boundary tests — registration and update paths.
//!
//! ## Size Budget Rationale
//!
//! Soroban persistent storage charges rent per byte. Every field stored on-chain
//! contributes to the ledger footprint and therefore to TTL extension costs.
//! The limits below are chosen to balance expressiveness with on-chain cost:
//!
//! | Field          | Limit  | Rationale                                            |
//! |----------------|--------|------------------------------------------------------|
//! | `name`         |  50 B  | Human-readable identifiers rarely exceed 50 chars    |
//! | `slug`         |  64 B  | URL slugs need a bit more room for namespacing       |
//! | `description`  | 2048 B | ~400 words; longer content belongs off-chain (IPFS)  |
//! | `category`     |  64 B  | Short taxonomy labels                                |
//! | `website`      | 256 B  | Standard URL length cap (RFC 7230 recommendation)    |
//! | `logo_cid`     | 128 B  | Max CIDv1 length (base32 multibase)                  |
//! | `metadata_cid` | 128 B  | Same as logo_cid                                     |
//! | `tag`          |  32 B  | Compact labels; up to 10 tags per project            |
//!
//! Tests at `max` must succeed; tests at `max + 1` must fail with the typed
//! error. Both registration (`register_project`) and update (`update_project`)
//! paths are exercised to ensure the same validation rules apply everywhere.

extern crate alloc;
use alloc::string::String as StdString;

use crate::constants::{
    MAX_CATEGORY_LEN, MAX_DESCRIPTION_LEN, MAX_NAME_LEN, MAX_SLUG_LEN, MAX_WEBSITE_LEN,
};
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use soroban_sdk::{testutils::Address as _, Address, Env, String as SStr};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn env() -> Env {
    Env::default()
}

fn rep(ch: char, n: usize) -> StdString {
    alloc::iter::repeat(ch).take(n).collect()
}

fn ss(env: &Env, s: &str) -> SStr {
    SStr::from_str(env, s)
}

fn ss_rep(env: &Env, ch: char, n: usize) -> SStr {
    SStr::from_str(env, &rep(ch, n))
}

/// Valid CIDv0 padded to `n` bytes starting with "Qm".
fn cid_of_len(env: &Env, n: usize) -> SStr {
    assert!(n >= 2);
    let mut s = StdString::from("Qm");
    s.push_str(&rep('a', n - 2));
    SStr::from_str(env, &s)
}

/// Valid CIDv1 of exactly `n` bytes (starts with 'b').
fn cidv1_of_len(env: &Env, n: usize) -> SStr {
    SStr::from_str(env, &rep('b', n))
}

fn base_params(env: &Env, owner: &Address, name: &str) -> ProjectRegistrationParams {
    let slug = name.to_lowercase().replace(' ', "-");
    ProjectRegistrationParams {
        owner: owner.clone(),
        name: ss(env, name),
        slug: ss(env, &slug),
        description: ss(env, "Default description for field limit tests."),
        category: ss(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// NAME — via registration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reg_name_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let name = rep('a', MAX_NAME_LEN);
    let slug = rep('a', MAX_NAME_LEN);
    let mut p = base_params(&e, &owner, "placeholder");
    p.name = ss(&e, &name);
    p.slug = ss(&e, &slug);
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin; // suppress unused warning
}

#[test]
fn reg_name_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "placeholder");
    p.name = ss_rep(&e, 'a', MAX_NAME_LEN + 1);
    p.slug = ss(&e, "valid-slug");
    let result = client.try_register_project(&p);
    assert_eq!(result, Err(Ok(ContractError::ProjectNameTooLong.into())));
    let _ = admin;
}

// ═══════════════════════════════════════════════════════════════════════════
// SLUG — via registration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reg_slug_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "Short");
    p.slug = ss_rep(&e, 'a', MAX_SLUG_LEN);
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn reg_slug_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "Short2");
    p.slug = ss_rep(&e, 'a', MAX_SLUG_LEN + 1);
    let result = client.try_register_project(&p);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData.into())));
    let _ = admin;
}

// ═══════════════════════════════════════════════════════════════════════════
// DESCRIPTION — via registration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reg_description_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "DescMax");
    p.description = ss_rep(&e, 'x', MAX_DESCRIPTION_LEN);
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn reg_description_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "DescOver");
    p.description = ss_rep(&e, 'x', MAX_DESCRIPTION_LEN + 1);
    let result = client.try_register_project(&p);
    assert_eq!(result, Err(Ok(ContractError::ProjectDescTooLong.into())));
    let _ = admin;
}

// ═══════════════════════════════════════════════════════════════════════════
// CATEGORY — via registration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reg_category_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "CatMax");
    p.category = ss_rep(&e, 'c', MAX_CATEGORY_LEN);
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn reg_category_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "CatOver");
    p.category = ss_rep(&e, 'c', MAX_CATEGORY_LEN + 1);
    let result = client.try_register_project(&p);
    assert_eq!(result, Err(Ok(ContractError::CategoryTooLong.into())));
    let _ = admin;
}

// ═══════════════════════════════════════════════════════════════════════════
// WEBSITE — via registration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reg_website_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let prefix = "https://";
    let url = alloc::format!("{}{}", prefix, rep('a', MAX_WEBSITE_LEN - prefix.len()));
    let mut p = base_params(&e, &owner, "WebMax");
    p.website = Some(ss(&e, &url));
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn reg_website_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let prefix = "https://";
    let url = alloc::format!("{}{}", prefix, rep('a', MAX_WEBSITE_LEN - prefix.len() + 1));
    let mut p = base_params(&e, &owner, "WebOver");
    p.website = Some(ss(&e, &url));
    let result = client.try_register_project(&p);
    assert_eq!(result, Err(Ok(ContractError::WebsiteTooLong.into())));
    let _ = admin;
}

// ═══════════════════════════════════════════════════════════════════════════
// LOGO CID — via registration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reg_logo_cid_at_max_128_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "LogoMax");
    p.logo_cid = Some(cidv1_of_len(&e, 128));
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn reg_logo_cid_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "LogoOver");
    // 129 bytes starting with 'b' — is_valid_ipfs_cid returns false (len > 128)
    p.logo_cid = Some(cidv1_of_len(&e, 129));
    let result = client.try_register_project(&p);
    assert_eq!(result, Err(Ok(ContractError::InvalidLogoCid.into())));
    let _ = admin;
}

#[test]
fn reg_logo_cid_at_min_46_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "LogoMin");
    p.logo_cid = Some(cid_of_len(&e, 46)); // CIDv0 minimum
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn reg_logo_cid_below_min_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "LogoShort");
    p.logo_cid = Some(cid_of_len(&e, 45)); // one below minimum
    let result = client.try_register_project(&p);
    assert_eq!(result, Err(Ok(ContractError::InvalidLogoCid.into())));
    let _ = admin;
}

// ═══════════════════════════════════════════════════════════════════════════
// METADATA CID — via registration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn reg_metadata_cid_at_max_128_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "MetaMax");
    p.metadata_cid = Some(cidv1_of_len(&e, 128));
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn reg_metadata_cid_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "MetaOver");
    p.metadata_cid = Some(cidv1_of_len(&e, 129));
    let result = client.try_register_project(&p);
    assert_eq!(result, Err(Ok(ContractError::InvalidMetaCid.into())));
    let _ = admin;
}

// ═══════════════════════════════════════════════════════════════════════════
// UPDATE PATH — same limits apply
// ═══════════════════════════════════════════════════════════════════════════

fn update_params(env: &Env, project_id: u64, caller: &Address) -> ProjectUpdateParams {
    ProjectUpdateParams {
        project_id,
        caller: caller.clone(),
        name: None,
        slug: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
    }
}

#[test]
fn update_description_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateDescMax");
    let mut p = update_params(&e, pid, &admin);
    p.description = Some(ss_rep(&e, 'x', MAX_DESCRIPTION_LEN));
    assert!(client.try_update_project(&p).is_ok());
}

#[test]
fn update_description_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateDescOver");
    let mut p = update_params(&e, pid, &admin);
    p.description = Some(ss_rep(&e, 'x', MAX_DESCRIPTION_LEN + 1));
    let result = client.try_update_project(&p);
    assert_eq!(result, Err(Ok(ContractError::ProjectDescTooLong.into())));
}

#[test]
fn update_category_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateCatMax");
    let mut p = update_params(&e, pid, &admin);
    p.category = Some(ss_rep(&e, 'c', MAX_CATEGORY_LEN));
    assert!(client.try_update_project(&p).is_ok());
}

#[test]
fn update_category_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateCatOver");
    let mut p = update_params(&e, pid, &admin);
    p.category = Some(ss_rep(&e, 'c', MAX_CATEGORY_LEN + 1));
    let result = client.try_update_project(&p);
    assert_eq!(result, Err(Ok(ContractError::CategoryTooLong.into())));
}

#[test]
fn update_website_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateWebMax");
    let prefix = "https://";
    let url = alloc::format!("{}{}", prefix, rep('a', MAX_WEBSITE_LEN - prefix.len()));
    let mut p = update_params(&e, pid, &admin);
    p.website = Some(Some(ss(&e, &url)));
    assert!(client.try_update_project(&p).is_ok());
}

#[test]
fn update_website_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateWebOver");
    let prefix = "https://";
    let url = alloc::format!("{}{}", prefix, rep('a', MAX_WEBSITE_LEN - prefix.len() + 1));
    let mut p = update_params(&e, pid, &admin);
    p.website = Some(Some(ss(&e, &url)));
    let result = client.try_update_project(&p);
    assert_eq!(result, Err(Ok(ContractError::WebsiteTooLong.into())));
}

#[test]
fn update_logo_cid_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateLogoMax");
    let mut p = update_params(&e, pid, &admin);
    p.logo_cid = Some(Some(cidv1_of_len(&e, 128)));
    assert!(client.try_update_project(&p).is_ok());
}

#[test]
fn update_logo_cid_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateLogoOver");
    let mut p = update_params(&e, pid, &admin);
    p.logo_cid = Some(Some(cidv1_of_len(&e, 129)));
    let result = client.try_update_project(&p);
    assert_eq!(result, Err(Ok(ContractError::InvalidLogoCid.into())));
}

#[test]
fn update_metadata_cid_at_max_accepted() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateMetaMax");
    let mut p = update_params(&e, pid, &admin);
    p.metadata_cid = Some(Some(cidv1_of_len(&e, 128)));
    assert!(client.try_update_project(&p).is_ok());
}

#[test]
fn update_metadata_cid_over_max_rejected() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let pid = create_test_project(&client, &admin, "UpdateMetaOver");
    let mut p = update_params(&e, pid, &admin);
    p.metadata_cid = Some(Some(cidv1_of_len(&e, 129)));
    let result = client.try_update_project(&p);
    assert_eq!(result, Err(Ok(ContractError::InvalidMetaCid.into())));
}

// ═══════════════════════════════════════════════════════════════════════════
// Boundary consistency: max passes, max+1 fails, max-1 passes
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn boundary_name_max_minus_one_passes() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let name = rep('a', MAX_NAME_LEN - 1);
    let mut p = base_params(&e, &owner, "x");
    p.name = ss(&e, &name);
    p.slug = ss(&e, &name);
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn boundary_description_max_minus_one_passes() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "BoundDesc");
    p.description = ss_rep(&e, 'x', MAX_DESCRIPTION_LEN - 1);
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn boundary_category_max_minus_one_passes() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "BoundCat");
    p.category = ss_rep(&e, 'c', MAX_CATEGORY_LEN - 1);
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn boundary_website_max_minus_one_passes() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let prefix = "https://";
    let url = alloc::format!("{}{}", prefix, rep('a', MAX_WEBSITE_LEN - prefix.len() - 1));
    let mut p = base_params(&e, &owner, "BoundWeb");
    p.website = Some(ss(&e, &url));
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}

#[test]
fn boundary_cid_at_127_passes() {
    let e = env();
    e.mock_all_auths();
    let (client, admin) = setup_contract(&e);
    let owner = Address::generate(&e);
    let mut p = base_params(&e, &owner, "BoundCid127");
    p.logo_cid = Some(cidv1_of_len(&e, 127));
    assert!(client.try_register_project(&p).is_ok());
    let _ = admin;
}
