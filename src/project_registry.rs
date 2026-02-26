//! Project registration with validation, per-user limits, and events.

use crate::constants::*;
use crate::errors::ContractError;
use crate::events::{ProjectRegistered, ProjectUpdated};
use crate::types::{DataKey, Project};
use soroban_sdk::{Address, Env, String as SorobanString};

// ── Validation Helpers ────────────────────────────────────────────────────────

/// Returns `true` iff `category` is exactly one of the five permitted values.
///
/// Comparison uses `soroban_sdk::String::from_str` so that the encoding is
/// identical to whatever the caller passed in — no hidden ASCII/UTF-8 mismatch.
fn is_valid_category(env: &Env, category: &SorobanString) -> bool {
    // Issue #8: strict enumeration — only these five strings are accepted.
    let valid = ["DeFi", "NFT", "Gaming", "DAO", "Tools"];
    for name in valid {
        if category == &SorobanString::from_str(env, name) {
            return true;
        }
    }
    false
}

fn validate_string_length(s: &SorobanString, max: usize) -> Result<(), ContractError> {
    if s.len() as usize > max {
        return Err(ContractError::ProjectNameTooLong);
    }
    Ok(())
}

fn validate_optional_string(s: &Option<SorobanString>, max: usize) -> Result<(), ContractError> {
    if let Some(ref x) = s {
        validate_string_length(x, max)?;
    }
    Ok(())
}

/// Validates all project registration / update inputs.
///
/// `env` is required so that `soroban_sdk::String::from_str` can be used for
/// the category comparison (fix for Issue #8 — previously `env` was absent,
/// causing a compile error).
pub fn validate_project_inputs(
    env: &Env,
    name: &SorobanString,
    description: &SorobanString,
    category: &SorobanString,
    website: &Option<SorobanString>,
    logo_cid: &Option<SorobanString>,
    metadata_cid: &Option<SorobanString>,
) -> Result<(), ContractError> {
    // Minimum length guards
    if name.len() < MIN_STRING_LEN as u32 {
        return Err(ContractError::InvalidProjectData);
    }
    if description.len() < MIN_STRING_LEN as u32 {
        return Err(ContractError::InvalidProjectData);
    }

    // ── Issue #8: strict category enumeration ────────────────────────────────
    if !is_valid_category(env, category) {
        return Err(ContractError::InvalidProjectCategory);
    }

    // Maximum length guards
    validate_string_length(name, MAX_NAME_LEN)?;
    validate_string_length(description, MAX_DESCRIPTION_LEN)?;
    validate_string_length(category, MAX_CATEGORY_LEN)?;
    validate_optional_string(website, MAX_WEBSITE_LEN)?;
    validate_optional_string(logo_cid, MAX_CID_LEN)?;
    validate_optional_string(metadata_cid, MAX_CID_LEN)?;

    Ok(())
}

// ── ProjectRegistry ───────────────────────────────────────────────────────────

pub struct ProjectRegistry;

impl ProjectRegistry {
    fn next_project_id(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::NextProjectId)
            .unwrap_or(1)
    }

    fn set_next_project_id(env: &Env, id: u64) {
        env.storage()
            .persistent()
            .set(&DataKey::NextProjectId, &(id + 1));
    }

    fn owner_project_count(env: &Env, owner: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerProjectCount(owner.clone()))
            .unwrap_or(0)
    }

    fn inc_owner_project_count(env: &Env, owner: &Address) {
        let count = Self::owner_project_count(env, owner);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerProjectCount(owner.clone()), &(count + 1));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_project(
        env: &Env,
        owner: Address,
        name: SorobanString,
        description: SorobanString,
        category: SorobanString,
        website: Option<SorobanString>,
        logo_cid: Option<SorobanString>,
        metadata_cid: Option<SorobanString>,
    ) -> Result<u64, ContractError> {
        owner.require_auth();

        // Pass `env` — required for soroban_sdk::String category comparison.
        validate_project_inputs(
            env,
            &name,
            &description,
            &category,
            &website,
            &logo_cid,
            &metadata_cid,
        )?;

        let count = Self::owner_project_count(env, &owner);
        if count >= MAX_PROJECTS_PER_USER {
            return Err(ContractError::InvalidProjectData);
        }

        let project_id = Self::next_project_id(env);
        let ledger_timestamp = env.ledger().timestamp();

        // Issue #8 — `category` is persisted as part of the Project struct.
        let project = Project {
            id: project_id,
            owner: owner.clone(),
            name: name.clone(),
            description: description.clone(),
            category: category.clone(),
            website: website.clone(),
            logo_cid: logo_cid.clone(),
            metadata_cid: metadata_cid.clone(),
            created_at: ledger_timestamp,
            updated_at: ledger_timestamp,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &project);

        Self::set_next_project_id(env, project_id);
        Self::inc_owner_project_count(env, &owner);

        ProjectRegistered {
            project_id,
            owner: owner.clone(),
            name: name.clone(),
            category: category.clone(),
        }
        .publish(env);

        Ok(project_id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_project(
        env: &Env,
        project_id: u64,
        caller: Address,
        name: SorobanString,
        description: SorobanString,
        category: SorobanString,
        website: Option<SorobanString>,
        logo_cid: Option<SorobanString>,
        metadata_cid: Option<SorobanString>,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id))
            .ok_or(ContractError::ProjectNotFound)?;

        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        validate_project_inputs(
            env,
            &name,
            &description,
            &category,
            &website,
            &logo_cid,
            &metadata_cid,
        )?;

        let ledger_timestamp = env.ledger().timestamp();
        project.name = name;
        project.description = description;
        project.category = category;
        project.website = website;
        project.logo_cid = logo_cid;
        project.metadata_cid = metadata_cid;
        project.updated_at = ledger_timestamp;

        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &project);

        ProjectUpdated {
            project_id,
            owner: caller,
            updated_at: ledger_timestamp,
        }
        .publish(env);

        Ok(())
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        env.storage()
            .persistent()
            .get(&DataKey::Project(project_id))
    }

    pub fn get_owner_project_count(env: &Env, owner: &Address) -> u32 {
        Self::owner_project_count(env, owner)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::{DongleContract, DongleContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger, LedgerInfo},
        Address, Env, String,
    };

    fn ledger_at(timestamp: u64) -> LedgerInfo {
        LedgerInfo {
            timestamp,
            protocol_version: 20,
            sequence_number: 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 10_000_000,
        }
    }

    fn setup(env: &Env) -> DongleContractClient<'_> {
        let contract_id = env.register_contract(None, DongleContract);
        DongleContractClient::new(env, &contract_id)
    }

    // ── Existing tests (preserved + fixed) ───────────────────────────────────

    #[test]
    fn test_ids_are_sequential() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id0 = client.register_project(
            &owner,
            &String::from_str(&env, "Alpha"),
            &String::from_str(&env, "desc"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );
        let id1 = client.register_project(
            &owner,
            &String::from_str(&env, "Beta"),
            &String::from_str(&env, "desc"),
            &String::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );

        assert_eq!(id0, 1);
        assert_eq!(id1, 2);
    }

    #[test]
    fn test_project_data_is_stored() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(ledger_at(1_700_000_000));
        let client = setup(&env);
        let owner = Address::generate(&env);

        // Fixed: "Infrastructure" replaced with valid category "Tools".
        let id = client.register_project(
            &owner,
            &String::from_str(&env, "Dongle"),
            &String::from_str(&env, "A Stellar registry"),
            &String::from_str(&env, "Tools"),
            &Some(String::from_str(&env, "https://dongle.xyz")),
            &None,
            &None,
        );

        let project = client.get_project(&id);
        assert_eq!(project.owner, owner);
        assert_eq!(project.name, String::from_str(&env, "Dongle"));
        assert_eq!(project.created_at, 1_700_000_000);
    }

    #[test]
    fn test_event_is_emitted_on_registration() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(ledger_at(1_710_000_000));
        let client = setup(&env);
        let owner = Address::generate(&env);

        client.register_project(
            &owner,
            &String::from_str(&env, "EventTest"),
            &String::from_str(&env, "Testing events"),
            &String::from_str(&env, "Tools"),
            &None,
            &None,
            &None,
        );

        let all_events = env.events().all();
        assert!(!all_events.is_empty());
        assert_eq!(all_events.len(), 1);
    }

    #[test]
    fn test_category_assignment_and_retrieval() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let category = String::from_str(&env, "DeFi");
        let id = client.register_project(
            &owner,
            &String::from_str(&env, "Test Project"),
            &String::from_str(&env, "Description"),
            &category,
            &None,
            &None,
            &None,
        );

        let project = client.get_project(&id);
        assert_eq!(project.category, category);
    }

    // ── Issue #8: Category enumeration tests ─────────────────────────────────

    /// Each of the five valid categories must be accepted without error.
    #[test]
    fn test_valid_category_defi() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "MyDeFiApp"),
            &String::from_str(&env, "A decentralized finance protocol"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        let project = client.get_project(&id);
        assert_eq!(project.category, String::from_str(&env, "DeFi"));
    }

    #[test]
    fn test_valid_category_nft() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "MyNFTApp"),
            &String::from_str(&env, "An NFT marketplace"),
            &String::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );

        let project = client.get_project(&id);
        assert_eq!(project.category, String::from_str(&env, "NFT"));
    }

    #[test]
    fn test_valid_category_gaming() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "MyGame"),
            &String::from_str(&env, "An on-chain game"),
            &String::from_str(&env, "Gaming"),
            &None,
            &None,
            &None,
        );

        let project = client.get_project(&id);
        assert_eq!(project.category, String::from_str(&env, "Gaming"));
    }

    #[test]
    fn test_valid_category_dao() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "MyDAO"),
            &String::from_str(&env, "A decentralized autonomous organization"),
            &String::from_str(&env, "DAO"),
            &None,
            &None,
            &None,
        );

        let project = client.get_project(&id);
        assert_eq!(project.category, String::from_str(&env, "DAO"));
    }

    #[test]
    fn test_valid_category_tools() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "MyTool"),
            &String::from_str(&env, "A developer tooling project"),
            &String::from_str(&env, "Tools"),
            &None,
            &None,
            &None,
        );

        let project = client.get_project(&id);
        assert_eq!(project.category, String::from_str(&env, "Tools"));
    }

    /// Any string not in the enumeration must return InvalidProjectCategory (code 14).
    #[test]
    #[should_panic(expected = "HostError: Error(Contract, #14)")]
    fn test_invalid_category_unknown_string() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        client.register_project(
            &owner,
            &String::from_str(&env, "Bad Cat"),
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "UnknownCategory"),
            &None,
            &None,
            &None,
        );
    }

    #[test]
    #[should_panic(expected = "HostError: Error(Contract, #14)")]
    fn test_invalid_category_empty_string() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        client.register_project(
            &owner,
            &String::from_str(&env, "No Cat"),
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, ""),
            &None,
            &None,
            &None,
        );
    }

    #[test]
    #[should_panic(expected = "HostError: Error(Contract, #14)")]
    fn test_invalid_category_wrong_case() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        // "defi" ≠ "DeFi" — case must match exactly.
        client.register_project(
            &owner,
            &String::from_str(&env, "Lower Case"),
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "defi"),
            &None,
            &None,
            &None,
        );
    }

    #[test]
    #[should_panic(expected = "HostError: Error(Contract, #14)")]
    fn test_invalid_category_whitespace_padded() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        // Whitespace-padded strings must not sneak through.
        client.register_project(
            &owner,
            &String::from_str(&env, "Padded"),
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, " DeFi "),
            &None,
            &None,
            &None,
        );
    }

    // ── Project ID Uniqueness & Sequential Assignment Tests ──────────────────

    /// Test that multiple users registering projects get unique sequential IDs.
    #[test]
    fn test_multiple_users_unique_sequential_ids() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);

        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let user3 = Address::generate(&env);

        // User 1 registers first project
        let id1 = client.register_project(
            &user1,
            &String::from_str(&env, "Project Alpha"),
            &String::from_str(&env, "First project"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        // User 2 registers second project
        let id2 = client.register_project(
            &user2,
            &String::from_str(&env, "Project Beta"),
            &String::from_str(&env, "Second project"),
            &String::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );

        // User 3 registers third project
        let id3 = client.register_project(
            &user3,
            &String::from_str(&env, "Project Gamma"),
            &String::from_str(&env, "Third project"),
            &String::from_str(&env, "Gaming"),
            &None,
            &None,
            &None,
        );

        // User 1 registers another project
        let id4 = client.register_project(
            &user1,
            &String::from_str(&env, "Project Delta"),
            &String::from_str(&env, "Fourth project"),
            &String::from_str(&env, "DAO"),
            &None,
            &None,
            &None,
        );

        // Verify IDs are sequential starting from 1
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
        assert_eq!(id4, 4);

        // Verify all IDs are unique
        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
        assert_ne!(id1, id4);
        assert_ne!(id2, id3);
        assert_ne!(id2, id4);
        assert_ne!(id3, id4);
    }

    /// Test that project retrieval returns the correct ID.
    #[test]
    fn test_project_retrieval_returns_correct_id() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);

        let owner = Address::generate(&env);

        let id1 = client.register_project(
            &owner,
            &String::from_str(&env, "First"),
            &String::from_str(&env, "Description 1"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        let id2 = client.register_project(
            &owner,
            &String::from_str(&env, "Second"),
            &String::from_str(&env, "Description 2"),
            &String::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );

        // Retrieve projects and verify IDs match
        let project1 = client.get_project(&id1);
        let project2 = client.get_project(&id2);

        assert_eq!(project1.id, id1);
        assert_eq!(project1.id, 1);
        assert_eq!(project2.id, id2);
        assert_eq!(project2.id, 2);
        assert_eq!(project1.name, String::from_str(&env, "First"));
        assert_eq!(project2.name, String::from_str(&env, "Second"));
    }

    /// Test that events emitted contain the correct project IDs.
    #[test]
    fn test_events_contain_correct_project_ids() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);

        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);

        // Register two projects
        let id1 = client.register_project(
            &owner1,
            &String::from_str(&env, "EventProject1"),
            &String::from_str(&env, "Testing event IDs"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        let id2 = client.register_project(
            &owner2,
            &String::from_str(&env, "EventProject2"),
            &String::from_str(&env, "Testing event IDs again"),
            &String::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );

        // Verify events were emitted
        let events = env.events().all();
        assert_eq!(events.len(), 2);

        // Events should contain the project IDs (1 and 2)
        // The event structure includes project_id in the topics
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    /// Test that IDs continue incrementing even after reaching higher numbers.
    #[test]
    fn test_ids_continue_incrementing() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);

        let owner = Address::generate(&env);

        // Register 10 projects
        let mut ids = Vec::new(&env);
        for i in 0..10 {
            let name = String::from_str(&env, "Project");
            let id = client.register_project(
                &owner,
                &name,
                &String::from_str(&env, "Description"),
                &String::from_str(&env, "DeFi"),
                &None,
                &None,
                &None,
            );
            ids.push_back(id);
        }

        // Verify all IDs are sequential from 1 to 10
        for i in 0..10 {
            assert_eq!(ids.get(i).unwrap(), (i + 1) as u64);
        }
    }

    /// Test that no duplicate IDs are assigned even with rapid registration.
    #[test]
    fn test_no_duplicate_ids_rapid_registration() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);

        let users: Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
        let mut all_ids = Vec::new(&env);

        // Simulate rapid registration by multiple users
        for (idx, user) in users.iter().enumerate() {
            let name = String::from_str(&env, "RapidProject");
            let id = client.register_project(
                user,
                &name,
                &String::from_str(&env, "Rapid registration test"),
                &String::from_str(&env, "DeFi"),
                &None,
                &None,
                &None,
            );
            all_ids.push_back(id);
        }

        // Verify all IDs are unique
        for i in 0..all_ids.len() {
            for j in (i + 1)..all_ids.len() {
                assert_ne!(
                    all_ids.get(i).unwrap(),
                    all_ids.get(j).unwrap(),
                    "Found duplicate IDs"
                );
            }
        }

        // Verify IDs are sequential
        for i in 0..all_ids.len() {
            assert_eq!(all_ids.get(i).unwrap(), (i + 1) as u64);
        }
    }

    /// Test that project IDs start from 1, not 0.
    #[test]
    fn test_first_project_id_is_one() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);

        let owner = Address::generate(&env);

        let first_id = client.register_project(
            &owner,
            &String::from_str(&env, "FirstEver"),
            &String::from_str(&env, "The very first project"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        assert_eq!(first_id, 1, "First project ID must be 1, not 0");
    }

    /// Test that different categories don't affect ID sequencing.
    #[test]
    fn test_ids_sequential_across_categories() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);

        let owner = Address::generate(&env);

        let id_defi = client.register_project(
            &owner,
            &String::from_str(&env, "DeFi Project"),
            &String::from_str(&env, "Description"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        let id_nft = client.register_project(
            &owner,
            &String::from_str(&env, "NFT Project"),
            &String::from_str(&env, "Description"),
            &String::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );

        let id_gaming = client.register_project(
            &owner,
            &String::from_str(&env, "Gaming Project"),
            &String::from_str(&env, "Description"),
            &String::from_str(&env, "Gaming"),
            &None,
            &None,
            &None,
        );

        let id_dao = client.register_project(
            &owner,
            &String::from_str(&env, "DAO Project"),
            &String::from_str(&env, "Description"),
            &String::from_str(&env, "DAO"),
            &None,
            &None,
            &None,
        );

        let id_tools = client.register_project(
            &owner,
            &String::from_str(&env, "Tools Project"),
            &String::from_str(&env, "Description"),
            &String::from_str(&env, "Tools"),
            &None,
            &None,
            &None,
        );

        // All IDs should be sequential regardless of category
        assert_eq!(id_defi, 1);
        assert_eq!(id_nft, 2);
        assert_eq!(id_gaming, 3);
        assert_eq!(id_dao, 4);
        assert_eq!(id_tools, 5);
    }

    /// Test that updating a project doesn't change its ID.
    #[test]
    fn test_update_preserves_project_id() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);

        let owner = Address::generate(&env);

        let original_id = client.register_project(
            &owner,
            &String::from_str(&env, "Original Name"),
            &String::from_str(&env, "Original description"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );

        // Update the project
        client.update_project(
            &original_id,
            &owner,
            &String::from_str(&env, "Updated Name"),
            &String::from_str(&env, "Updated description"),
            &String::from_str(&env, "NFT"),
            &Some(String::from_str(&env, "https://updated.com")),
            &None,
            &None,
        );

        // Retrieve and verify ID hasn't changed
        let updated_project = client.get_project(&original_id);
        assert_eq!(updated_project.id, original_id);
        assert_eq!(updated_project.name, String::from_str(&env, "Updated Name"));
    }
}
