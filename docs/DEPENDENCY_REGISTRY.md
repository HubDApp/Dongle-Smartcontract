# Dependency Registry

Projects registered in the contract can declare **dependencies** on other
references — another registered project, an IPFS CID, an external URL, or an
external Stellar contract address. This document describes the rules that
protect the project-to-project dependency graph from circular references and
runaway nesting.

Relevant source: `dongle-smartcontract/src/dependency_registry.rs`,
`constants::MAX_DEPENDENCY_DEPTH`.

## Reference kinds

`DependencyRef` carries exactly one of:

| Field | Meaning | Participates in the graph? |
| --- | --- | --- |
| `project_id` | Another project inside this contract | **Yes** |
| `external_cid` | Content-addressed reference (IPFS CID) | No |
| `external_url` | `http://` / `https://` URL | No |
| `external_contract` | 56-char Stellar contract address (`C…`) | No |

Only `project_id` references can form a cycle inside the contract, so the
circular-reference and depth checks apply **only** to `project_id`
references. External references are opaque pointers and are never traversed.

## Rules enforced on `add_project_dependency`

When the new dependency's `reference.project_id` is set, the registry runs
three checks before storing the edge (`dependent → target`):

1. **No self-reference.** `target == dependent` is rejected with
   `CannotLinkToSelf` (error 45).
2. **No circular reference.** Starting from `target`, the registry walks the
   transitive dependency graph breadth-first. If the walk reaches
   `dependent`, the new edge would close a cycle and is rejected with
   `CircularDependency` (error 75).
3. **Depth limit.** The edge `dependent → target` is level 1. Each further
   hop is one level deeper. If the walk still has unexplored nodes beyond
   `MAX_DEPENDENCY_DEPTH` levels, the edge is rejected with
   `DependencyDepthExceeded` (error 76).

`MAX_DEPENDENCY_DEPTH` is **5**. It is defined in
`dongle-smartcontract/src/constants.rs` and can be changed there in a future
release; the value is deliberately small to keep the on-chain graph walk
bounded and cheap.

### Worked examples

Assume every edge below is a `project_id` reference.

| Existing graph | Action | Result |
| --- | --- | --- |
| *(none)* | `A` adds dependency on `A` | `CannotLinkToSelf` |
| `B → A` | `A` adds dependency on `B` | `CircularDependency` (`B` already depends on `A`) |
| `B → C → A` | `A` adds dependency on `B` | `CircularDependency` (transitive) |
| `A → B → C → D → E` | `E` adds dependency on `F` (chain becomes `A…F`, 5 hops) | OK — exactly 5 levels |
| `A → B → C → D → E → F` | `F` adds dependency on `G` (6 hops) | `DependencyDepthExceeded` |
| `A → B` | `A` adds dependency on `B` again | `AlreadyLinked` |

Note that the depth check is evaluated from the perspective of the project
being edited. Two independently valid chains can still be rejected when
joining them would exceed the limit.

## Update and remove

- `update_project_dependency` only updates the metadata of an existing
  dependency (`label`, `metadata_cid`). It keeps the original reference
  identity, so it cannot change a `project_id` target and therefore cannot
  introduce a cycle. No graph check is performed.
- `remove_project_dependency` always shrinks the graph and is never blocked
  by these rules.

## Operational notes

- The checks read other projects' dependency key-lists from persistent
  storage. Cost scales with the size of the reachable sub-graph, which is
  bounded by `MAX_DEPENDENCY_DEPTH` levels and the fan-out per project.
- Because the graph can only be extended through `add_project_dependency`,
  and every addition is checked, the stored graph is always acyclic and at
  most `MAX_DEPENDENCY_DEPTH` levels deep.
- A project that is deleted/archived while other projects depend on it
  leaves a dangling `project_id` reference; readers should treat an
  unresolvable `project_id` as an external/unknown dependency.

## Reading the graph

- `get_project_dependencies(project_id)` — all dependency records for a
  project.
- `get_dependency_count(project_id)` — O(1) count for badges/UI.

There is no built-in transitive-closure getter; clients that need the full
tree should walk `get_project_dependencies` recursively (the depth limit
guarantees the walk terminates quickly).
