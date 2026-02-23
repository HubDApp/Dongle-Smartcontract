use crate::types::Project;
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Address, Map, String};

#[contracttype]
pub enum DataKey {
    Projects,
    NextProjectId,
}

pub struct ProjectRegistry;

impl ProjectRegistry {
    pub fn register_project(
        env: &Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> u64 {
        owner.require_auth();

        let project_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextProjectId)
            .unwrap_or(0u64);

        let registered_at: u64 = env.ledger().timestamp();

        let project = Project {
            id: project_id,
            owner: owner.clone(),
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
            registered_at,
        };

        let mut projects: Map<u64, Project> = env
            .storage()
            .persistent()
            .get(&DataKey::Projects)
            .unwrap_or(Map::new(env));

        projects.set(project_id, project);
        env.storage().persistent().set(&DataKey::Projects, &projects);
        env.storage()
            .persistent()
            .set(&DataKey::NextProjectId, &(project_id + 1));

        env.events().publish(
            (symbol_short!("ProjReg"), owner.clone()),
            (project_id, registered_at),
        );

        project_id
    }

    pub fn update_project(
        env: &Env,
        project_id: u64,
        caller: Address,
        name: Option<String>,
        description: Option<String>,
        category: Option<String>,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) {
        caller.require_auth();

        let mut projects: Map<u64, Project> = env
            .storage()
            .persistent()
            .get(&DataKey::Projects)
            .unwrap_or(Map::new(env));

        let mut project = projects.get(project_id).expect("project not found");
        assert!(project.owner == caller, "only the owner can update");

        if let Some(v) = name        { project.name        = v; }
        if let Some(v) = description { project.description = v; }
        if let Some(v) = category    { project.category    = v; }
        project.website      = website;
        project.logo_cid     = logo_cid;
        project.metadata_cid = metadata_cid;

        projects.set(project_id, project);
        env.storage().persistent().set(&DataKey::Projects, &projects);
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        let projects: Map<u64, Project> = env
            .storage()
            .persistent()
            .get(&DataKey::Projects)
            .unwrap_or(Map::new(env));

        projects.get(project_id)
    }
}

// ── Contract wrapper required for test context ────────────────────────────────

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    pub fn register_project(
        env: Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> u64 {
        ProjectRegistry::register_project(
            &env, owner, name, description, category, website, logo_cid, metadata_cid,
        )
    }

    pub fn update_project(
        env: Env,
        project_id: u64,
        caller: Address,
        name: Option<String>,
        description: Option<String>,
        category: Option<String>,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) {
        ProjectRegistry::update_project(
            &env, project_id, caller, name, description, category, website, logo_cid, metadata_cid,
        )
    }

    pub fn get_project(env: Env, project_id: u64) -> Option<Project> {
        ProjectRegistry::get_project(&env, project_id)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger, LedgerInfo},
        vec, Address, Env, IntoVal, TryIntoVal,
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

    // Registers the contract and returns a client to call it through
    fn setup(env: &Env) -> DongleContractClient {
        let contract_id = env.register_contract(None, DongleContract);
        DongleContractClient::new(env, &contract_id)
    }

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
            &None, &None, &None,
        );
        let id1 = client.register_project(
            &owner,
            &String::from_str(&env, "Beta"),
            &String::from_str(&env, "desc"),
            &String::from_str(&env, "NFT"),
            &None, &None, &None,
        );

        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
    }

    #[test]
    fn test_project_data_is_stored() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(ledger_at(1_700_000_000));
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "Dongle"),
            &String::from_str(&env, "A Stellar registry"),
            &String::from_str(&env, "Infrastructure"),
            &Some(String::from_str(&env, "https://dongle.xyz")),
            &None,
            &None,
        );

        let project = client.get_project(&id).unwrap();
        assert_eq!(project.owner, owner);
        assert_eq!(project.name, String::from_str(&env, "Dongle"));
        assert_eq!(project.registered_at, 1_700_000_000);
    }

    #[test]
    fn test_event_is_emitted_on_registration() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(ledger_at(1_710_000_000));
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "EventTest"),
            &String::from_str(&env, "Testing events"),
            &String::from_str(&env, "Testing"),
            &None, &None, &None,
        );

        let all_events = env.events().all();
        assert!(!all_events.is_empty());

        let (_, topics, data) = all_events.last().unwrap();

        let expected_topics = vec![
            &env,
            symbol_short!("ProjReg").into_val(&env),
            owner.into_val(&env),
        ];
        assert_eq!(topics, expected_topics);

        let (emitted_id, emitted_ts): (u64, u64) = data.try_into_val(&env).unwrap();
        assert_eq!(emitted_id, id);
        assert_eq!(emitted_ts, 1_710_000_000u64);
    }

    #[test]
    fn test_one_event_per_registration() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        for _ in 0..3 {
            client.register_project(
                &owner,
                &String::from_str(&env, "P"),
                &String::from_str(&env, "d"),
                &String::from_str(&env, "c"),
                &None, &None, &None,
            );
        }

        assert_eq!(env.events().all().len(), 3);
    }
}