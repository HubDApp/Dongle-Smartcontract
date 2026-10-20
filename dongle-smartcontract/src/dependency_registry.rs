use crate::errors::ContractError;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::storage_manager::StorageManager;
use crate::types::{
    DependencyGraph, DependencyGraphEdge, DependencyGraphNode, DependencyRef, DependencyRelation,
    ProjectDependency,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Map, String, Vec};

pub struct DependencyRegistry;

impl DependencyRegistry {
    fn normalize_url(url: &String) -> Result<(), ContractError> {
        let len = url.len();
        if len == 0 || len > crate::constants::MAX_SOCIAL_LINK_URL_LEN as u32 {
            return Err(ContractError::InvalidInput);
        }
        let mut buf = [0u8; crate::constants::MAX_SOCIAL_LINK_URL_LEN];
        let slice = &mut buf[..len as usize];
        url.copy_into_slice(slice);
        if !slice.starts_with(b"http://") && !slice.starts_with(b"https://") {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    /// Validate a Stellar contract address: 56 uppercase base32 chars starting with 'C'.
    fn validate_contract_address(addr: &String) -> Result<(), ContractError> {
        let len = addr.len();
        if len != 56 {
            return Err(ContractError::InvalidProjectData);
        }
        let mut buf = [0u8; 56];
        addr.copy_into_slice(&mut buf);
        if buf[0] != b'C' {
            return Err(ContractError::InvalidProjectData);
        }
        for &c in buf.iter() {
            if !(c.is_ascii_uppercase() || (b'2'..=b'7').contains(&c)) {
                return Err(ContractError::InvalidProjectData);
            }
        }
        Ok(())
    }

    fn validate_dependency_ref(env: &Env, dep: &DependencyRef) -> Result<(), ContractError> {
        // Exactly one reference kind must be set.
        if !Utils::exactly_one_some([
            dep.project_id.map(|_| ()),
            dep.external_cid.as_ref().map(|_| ()),
            dep.external_url.as_ref().map(|_| ()),
            dep.external_contract.as_ref().map(|_| ()),
        ]) {
            return Err(ContractError::InvalidProjectData);
        }

        if let Some(cid) = &dep.external_cid {
            Utils::validate_metadata_cid(cid).map_err(|_| ContractError::InvalidCid)?;
        }

        if let Some(url) = &dep.external_url {
            Self::normalize_url(url)?;
        }

        if let Some(project_id) = dep.project_id {
            if !env
                .storage()
                .persistent()
                .has(&StorageKey::Project(project_id))
            {
                return Err(ContractError::ProjectNotFound);
            }
        }

        if let Some(contract) = &dep.external_contract {
            Self::validate_contract_address(contract)?;
        }

        Ok(())
    }

    fn dependency_key(env: &Env, dep: &DependencyRef) -> Result<String, ContractError> {
        if let Some(pid) = dep.project_id {
            let mut num_buf = [0u8; 20];
            let mut val = pid;
            let mut idx = 20;
            if val == 0 {
                idx -= 1;
                num_buf[idx] = b'0';
            } else {
                while val > 0 {
                    idx -= 1;
                    num_buf[idx] = b'0' + (val % 10) as u8;
                    val /= 10;
                }
            }
            let num_len = 20 - idx;
            let mut buf = [0u8; 24];
            buf[0..4].copy_from_slice(b"PID:");
            buf[4..4 + num_len].copy_from_slice(&num_buf[idx..20]);
            let key_str = core::str::from_utf8(&buf[..4 + num_len])
                .map_err(|_| ContractError::InvalidProjectData)?;
            return Ok(String::from_str(env, key_str));
        }
        if let Some(cid) = &dep.external_cid {
            let cid_len = cid.len();
            if cid_len > crate::constants::MAX_CID_LEN as u32 {
                return Err(ContractError::InvalidProjectData);
            }
            let mut buf = [0u8; 4 + 128]; // "CID:" (4) + max cid (128)
            buf[0..4].copy_from_slice(b"CID:");
            cid.copy_into_slice(&mut buf[4..4 + cid_len as usize]);
            let key_str = core::str::from_utf8(&buf[..4 + cid_len as usize])
                .map_err(|_| ContractError::InvalidProjectData)?;
            return Ok(String::from_str(env, key_str));
        }
        if let Some(url) = &dep.external_url {
            let url_len = url.len();
            if url_len > crate::constants::MAX_SOCIAL_LINK_URL_LEN as u32 {
                return Err(ContractError::InvalidProjectData);
            }
            let mut buf = [0u8; 4 + 256]; // "URL:" (4) + max url (256)
            buf[0..4].copy_from_slice(b"URL:");
            url.copy_into_slice(&mut buf[4..4 + url_len as usize]);
            let key_str = core::str::from_utf8(&buf[..4 + url_len as usize])
                .map_err(|_| ContractError::InvalidProjectData)?;
            return Ok(String::from_str(env, key_str));
        }
        if let Some(contract) = &dep.external_contract {
            // Contract addresses must be exactly 56 chars; "CTR:" prefix ensures no collision.
            if contract.len() != 56 {
                return Err(ContractError::InvalidProjectData);
            }
            let mut buf = [0u8; 4 + 56];
            buf[0..4].copy_from_slice(b"CTR:");
            contract.copy_into_slice(&mut buf[4..60]);
            let key_str =
                core::str::from_utf8(&buf[..60]).map_err(|_| ContractError::InvalidProjectData)?;
            return Ok(String::from_str(env, key_str));
        }
        Err(ContractError::InvalidProjectData)
    }

    /// Collect the `project_id` targets of a project's registered
    /// dependencies. External references (cid / url / contract) are ignored
    /// because they cannot form a cycle inside this contract's graph.
    fn dependency_project_ids(env: &Env, project_id: u64) -> Vec<u64> {
        let mut out = Vec::new(env);
        let keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        for i in 0..keys.len() {
            if let Some(k) = keys.get(i) {
                if let Some(dep) = env
                    .storage()
                    .persistent()
                    .get::<_, ProjectDependency>(&ExtensionKey::ProjectDependency(project_id, k))
                {
                    if let Some(pid) = dep.reference.project_id {
                        out.push_back(pid);
                    }
                }
            }
        }
        out
    }

    /// Guard against circular references and excessive nesting before a
    /// project-to-project dependency edge (`dependent` -> `new_dep`) is stored.
    ///
    /// The edge `dependent -> new_dep` is treated as level 1. Starting from
    /// `new_dep`, the transitive dependency graph is walked breadth-first:
    /// - reaching `dependent` again means the edge would close a cycle
    ///   (`CircularDependency`);
    /// - a walk that still has unexplored nodes beyond
    ///   `MAX_DEPENDENCY_DEPTH` levels means the chain is too deep
    ///   (`DependencyDepthExceeded`).
    fn check_project_dependency_graph(
        env: &Env,
        dependent: u64,
        new_dep: u64,
    ) -> Result<(), ContractError> {
        if new_dep == dependent {
            return Err(ContractError::CannotLinkToSelf);
        }

        let mut frontier: Vec<u64> = Vec::new(env);
        frontier.push_back(new_dep);
        let mut visited: Vec<u64> = Vec::new(env);
        visited.push_back(new_dep);

        let mut level: u32 = 1;
        while frontier.len() > 0 {
            if level > crate::constants::MAX_DEPENDENCY_DEPTH {
                return Err(ContractError::DependencyDepthExceeded);
            }
            let mut next: Vec<u64> = Vec::new(env);
            for i in 0..frontier.len() {
                if let Some(node) = frontier.get(i) {
                    if node == dependent {
                        return Err(ContractError::CircularDependency);
                    }
                    let children = Self::dependency_project_ids(env, node);
                    for j in 0..children.len() {
                        if let Some(child) = children.get(j) {
                            if !visited.contains(child) {
                                visited.push_back(child);
                                next.push_back(child);
                            }
                        }
                    }
                }
            }
            frontier = next;
            level += 1;
        }

        Ok(())
    }

    pub fn add_dependency(
        env: &Env,
        project_id: u64,
        caller: Address,
        dependency: ProjectDependency,
    ) -> Result<(), ContractError> {
        let project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }
        Self::validate_dependency_ref(env, &dependency.reference)?;

        if let Some(dep_pid) = dependency.reference.project_id {
            Self::check_project_dependency_graph(env, project_id, dep_pid)?;
        }

        let key = Self::dependency_key(env, &dependency.reference)?;

        // Reject duplicates
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::AlreadyLinked);
        }

        env.storage().persistent().set(
            &ExtensionKey::ProjectDependency(project_id, key.clone()),
            &dependency,
        );

        // Track key list for fetch.
        let mut keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        keys.push_back(key);
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectDependencyKeys(project_id), &keys);

        // Update reverse dependency index if it's a project reference
        if let Some(dep_pid) = dependency.reference.project_id {
            let mut dependents: Vec<u64> = env
                .storage()
                .persistent()
                .get(&crate::storage_keys::ExtensionKey2::ProjectDependents(dep_pid))
                .unwrap_or_else(|| Vec::new(env));
            if !dependents.contains(&project_id) {
                dependents.push_back(project_id);
                env.storage()
                    .persistent()
                    .set(&crate::storage_keys::ExtensionKey2::ProjectDependents(dep_pid), &dependents);
            }
        }

        StorageManager::extend_project_dependency_ttl(env, project_id);
        Ok(())
    }

    pub fn update_dependency(
        env: &Env,
        project_id: u64,
        caller: Address,
        dependency_key: DependencyRef,
        new_dependency: ProjectDependency,
    ) -> Result<(), ContractError> {
        let project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        Self::validate_dependency_ref(env, &dependency_key)?;
        Self::validate_dependency_ref(env, &new_dependency.reference)?;

        let key = Self::dependency_key(env, &dependency_key)?;

        if !env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::ProjectNotFound);
        }

        // Keep same key slot; allow updating metadata.
        let mut stored = new_dependency;
        // Ensure dependency reference doesn't change the identity slot (still uses key)
        stored.reference = dependency_key;

        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectDependency(project_id, key), &stored);

        StorageManager::extend_project_dependency_ttl(env, project_id);
        Ok(())
    }

    pub fn remove_dependency(
        env: &Env,
        project_id: u64,
        caller: Address,
        dependency_key: DependencyRef,
    ) -> Result<(), ContractError> {
        let project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        Self::validate_dependency_ref(env, &dependency_key)?;
        let key = Self::dependency_key(env, &dependency_key)?;

        if !env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::ProjectNotFound);
        }

        env.storage()
            .persistent()
            .remove(&ExtensionKey::ProjectDependency(project_id, key.clone()));

        let keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        let new_keys = Utils::remove_item_from_vec(env, &keys, &key);
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectDependencyKeys(project_id), &new_keys);

        // Update reverse dependency index if it's a project reference
        if let Some(dep_pid) = dependency_key.project_id {
            let dependents: Vec<u64> = env
                .storage()
                .persistent()
                .get(&crate::storage_keys::ExtensionKey2::ProjectDependents(dep_pid))
                .unwrap_or_else(|| Vec::new(env));
            let mut new_dependents = Vec::new(env);
            for i in 0..dependents.len() {
                if let Some(d) = dependents.get(i) {
                    if d != project_id {
                        new_dependents.push_back(d);
                    }
                }
            }
            env.storage()
                .persistent()
                .set(&crate::storage_keys::ExtensionKey2::ProjectDependents(dep_pid), &new_dependents);
        }

        StorageManager::extend_project_dependency_ttl(env, project_id);
        Ok(())
    }

    pub fn get_dependencies(env: &Env, project_id: u64) -> Vec<ProjectDependency> {
        let mut out = Vec::new(env);
        let keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));

        for i in 0..keys.len() {
            if let Some(k) = keys.get(i) {
                let dep = env
                    .storage()
                    .persistent()
                    .get(&ExtensionKey::ProjectDependency(project_id, k));
                if let Some(d) = dep {
                    out.push_back(d);
                }
            }
        }
        StorageManager::extend_project_dependency_ttl(env, project_id);
        out
    }

    /// Returns the number of dependencies registered for a project.
    ///
    /// This is a cheap O(1) operation — it reads only the key-list length
    /// without fetching individual dependency records, making it suitable for
    /// displaying a dependency-count badge in UIs.
    pub fn get_dependency_count(env: &Env, project_id: u64) -> u32 {
        let keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        keys.len()
    }

    pub fn get_dependent_projects(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&crate::storage_keys::ExtensionKey2::ProjectDependents(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn get_transitive_dependencies(env: &Env, project_id: u64) -> Vec<u64> {
        let mut out: Vec<u64> = Vec::new(env);
        let mut frontier: Vec<u64> = Vec::new(env);
        frontier.push_back(project_id);
        
        let mut level = 0;
        // Limit depth to avoid running out of gas/instructions, cycle detection is handled,
        // but large graphs still need limits on reads
        while frontier.len() > 0 && level < crate::constants::MAX_DEPENDENCY_DEPTH {
            let mut next_frontier = Vec::new(env);
            for i in 0..frontier.len() {
                if let Some(node) = frontier.get(i) {
                    let children = Self::dependency_project_ids(env, node);
                    for j in 0..children.len() {
                        if let Some(child) = children.get(j) {
                            if !out.contains(child) {
                                out.push_back(child);
                                next_frontier.push_back(child);
                            }
                        }
                    }
                }
            }
            frontier = next_frontier;
            level += 1;
        }
        out
    }

    /// Build a typed dependency graph rooted at `project_id` (issue #924).
    ///
    /// The graph combines three relationship types:
    /// - `DependsOn` edges, walked transitively downstream from the root;
    /// - `UsedBy` edges, walked transitively upstream (reverse dependency
    ///   index) from the root;
    /// - `RelatedTo` edges for the root's linked projects (one hop only).
    ///
    /// `max_depth` limits how many hops each transitive walk may take. `0`
    /// or any value above `MAX_DEPENDENCY_GRAPH_DEPTH` is clamped to that
    /// maximum; `1` returns direct relationships only. At most
    /// `MAX_DEPENDENCY_GRAPH_NODES` nodes are returned; `truncated` reports
    /// whether either limit cut the graph short.
    ///
    /// Cycle detection runs over the returned `DependsOn` / `UsedBy` edges,
    /// so a cycle introduced by legacy or migrated data is surfaced in
    /// `has_cycle` / `cycle_nodes` instead of looping the traversal.
    pub fn get_dependency_graph(
        env: &Env,
        project_id: u64,
        max_depth: u32,
    ) -> Result<DependencyGraph, ContractError> {
        if !env
            .storage()
            .persistent()
            .has(&StorageKey::Project(project_id))
        {
            return Err(ContractError::ProjectNotFound);
        }

        let depth_limit = if max_depth == 0 || max_depth > crate::constants::MAX_DEPENDENCY_GRAPH_DEPTH
        {
            crate::constants::MAX_DEPENDENCY_GRAPH_DEPTH
        } else {
            max_depth
        };

        let mut nodes: Vec<DependencyGraphNode> = Vec::new(env);
        let mut seen: Map<u64, u32> = Map::new(env);
        let mut edges: Vec<DependencyGraphEdge> = Vec::new(env);
        let mut truncated = false;

        nodes.push_back(DependencyGraphNode {
            project_id,
            depth: 0,
        });
        seen.set(project_id, 0);

        Self::walk_graph(
            env,
            project_id,
            depth_limit,
            DependencyRelation::DependsOn,
            &mut nodes,
            &mut seen,
            &mut edges,
            &mut truncated,
        );
        Self::walk_graph(
            env,
            project_id,
            depth_limit,
            DependencyRelation::UsedBy,
            &mut nodes,
            &mut seen,
            &mut edges,
            &mut truncated,
        );

        let linked = crate::project_registry::ProjectRegistry::get_linked_projects(env, project_id);
        for linked_id in linked.iter() {
            if linked_id == project_id {
                continue;
            }
            if !seen.contains_key(linked_id) {
                if nodes.len() >= crate::constants::MAX_DEPENDENCY_GRAPH_NODES {
                    truncated = true;
                    continue;
                }
                nodes.push_back(DependencyGraphNode {
                    project_id: linked_id,
                    depth: 1,
                });
                seen.set(linked_id, 1);
            }
            edges.push_back(DependencyGraphEdge {
                from: project_id,
                to: linked_id,
                relation: DependencyRelation::RelatedTo,
            });
        }

        let cycle_nodes = Self::find_cycle_nodes(env, &edges);

        Ok(DependencyGraph {
            root: project_id,
            max_depth: depth_limit,
            nodes,
            edges,
            has_cycle: !cycle_nodes.is_empty(),
            cycle_nodes,
            truncated,
        })
    }

    /// Breadth-first walk from `root` in one direction: downstream along
    /// declared dependencies (`DependsOn`) or upstream along the reverse
    /// dependency index (`UsedBy`). Edges to already-visited projects are
    /// still recorded so cycles remain visible to `find_cycle_nodes`.
    #[allow(clippy::too_many_arguments)]
    fn walk_graph(
        env: &Env,
        root: u64,
        depth_limit: u32,
        relation: DependencyRelation,
        nodes: &mut Vec<DependencyGraphNode>,
        seen: &mut Map<u64, u32>,
        edges: &mut Vec<DependencyGraphEdge>,
        truncated: &mut bool,
    ) {
        let neighbours = |node: u64| -> Vec<u64> {
            if relation == DependencyRelation::DependsOn {
                Self::dependency_project_ids(env, node)
            } else {
                Self::get_dependent_projects(env, node)
            }
        };

        let mut visited: Map<u64, bool> = Map::new(env);
        visited.set(root, true);
        let mut frontier: Vec<u64> = Vec::new(env);
        frontier.push_back(root);

        let mut level: u32 = 1;
        while !frontier.is_empty() {
            if level > depth_limit {
                // Anything still reachable past the limit means the export
                // is incomplete.
                for node in frontier.iter() {
                    if !neighbours(node).is_empty() {
                        *truncated = true;
                        break;
                    }
                }
                break;
            }

            let mut next: Vec<u64> = Vec::new(env);
            for node in frontier.iter() {
                for child in neighbours(node).iter() {
                    if !seen.contains_key(child) {
                        if nodes.len() >= crate::constants::MAX_DEPENDENCY_GRAPH_NODES {
                            *truncated = true;
                            continue;
                        }
                        nodes.push_back(DependencyGraphNode {
                            project_id: child,
                            depth: level,
                        });
                        seen.set(child, level);
                    }
                    edges.push_back(DependencyGraphEdge {
                        from: node,
                        to: child,
                        relation,
                    });
                    if !visited.contains_key(child) {
                        visited.set(child, true);
                        next.push_back(child);
                    }
                }
            }
            frontier = next;
            level += 1;
        }
    }

    /// Return the projects that lie on (or between) cycles of the
    /// dependency relation described by `edges`.
    ///
    /// `UsedBy(a, b)` is normalised to `b depends on a`; `RelatedTo` edges
    /// carry no dependency and are ignored. Nodes with no incoming or no
    /// outgoing dependency edge cannot be on a cycle, so they are peeled off
    /// repeatedly; whatever remains participates in a cycle.
    fn find_cycle_nodes(env: &Env, edges: &Vec<DependencyGraphEdge>) -> Vec<u64> {
        let mut pairs: Map<(u64, u64), bool> = Map::new(env);
        for edge in edges.iter() {
            let pair = match edge.relation {
                DependencyRelation::DependsOn => (edge.from, edge.to),
                DependencyRelation::UsedBy => (edge.to, edge.from),
                DependencyRelation::RelatedTo => continue,
            };
            pairs.set(pair, true);
        }

        let mut successors: Map<u64, Vec<u64>> = Map::new(env);
        let mut predecessors: Map<u64, Vec<u64>> = Map::new(env);
        let mut in_degree: Map<u64, u32> = Map::new(env);
        let mut out_degree: Map<u64, u32> = Map::new(env);
        for (from, to) in pairs.keys().iter() {
            let mut succ = successors.get(from).unwrap_or_else(|| Vec::new(env));
            succ.push_back(to);
            successors.set(from, succ);
            let mut pred = predecessors.get(to).unwrap_or_else(|| Vec::new(env));
            pred.push_back(from);
            predecessors.set(to, pred);
            out_degree.set(from, out_degree.get(from).unwrap_or(0) + 1);
            in_degree.set(to, in_degree.get(to).unwrap_or(0) + 1);
            if !in_degree.contains_key(from) {
                in_degree.set(from, 0);
            }
            if !out_degree.contains_key(to) {
                out_degree.set(to, 0);
            }
        }

        let all_nodes = in_degree.keys();
        let mut removed: Map<u64, bool> = Map::new(env);
        loop {
            let mut changed = false;
            for node in all_nodes.iter() {
                if removed.contains_key(node) {
                    continue;
                }
                if in_degree.get(node).unwrap_or(0) > 0 && out_degree.get(node).unwrap_or(0) > 0 {
                    continue;
                }
                removed.set(node, true);
                changed = true;
                for succ in successors.get(node).unwrap_or_else(|| Vec::new(env)).iter() {
                    let d = in_degree.get(succ).unwrap_or(0);
                    in_degree.set(succ, d.saturating_sub(1));
                }
                for pred in predecessors.get(node).unwrap_or_else(|| Vec::new(env)).iter() {
                    let d = out_degree.get(pred).unwrap_or(0);
                    out_degree.set(pred, d.saturating_sub(1));
                }
            }
            if !changed {
                break;
            }
        }

        let mut cycle_nodes = Vec::new(env);
        for node in all_nodes.iter() {
            if !removed.contains_key(node) {
                cycle_nodes.push_back(node);
            }
        }
        cycle_nodes
    }
}
