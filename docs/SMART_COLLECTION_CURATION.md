# Smart Collections — Rule-Based Curation

How the rule-based ("smart") collections of issue #906 work: how a curation
rule is evaluated, how the membership is (re)computed, and how rule versions
and rollbacks are handled.

Audience: admins, curators and indexers. Pair this with
[COLLECTION_MEMBERSHIP_VOTING.md](COLLECTION_MEMBERSHIP_VOTING.md) (community
voting on membership) and [STORAGE_SCHEMA.md](STORAGE_SCHEMA.md).

---

## 1. What a smart collection is

A regular collection (`src/collection_registry.rs`) has a hand-picked member
list. A **smart collection** derives its member list from declarative
**curation rules** instead, so the collection keeps itself up to date as
projects change their tags, ratings or categories.

A rule is a conjunction of the conditions that are set:

| Condition | Meaning |
|---|---|
| `tag` | the project must carry this tag (case-insensitive match) |
| `min_rating` | the project's average rating must be at least this value |
| `category` | the project must belong to this category (case-insensitive match) |

* A rule with **no** condition is rejected (`InvalidCurationRule`).
* A rule with several conditions requires **all** of them to hold.
* A project matches the collection when it matches **at least one** rule.

`min_rating` uses the scaled rating format of `ProjectStats::average_rating`
(`350` = 3.5 stars, `500` = 5.0 stars). A project with no reviews has an
average rating of `0` and therefore never matches a `min_rating` rule.

---

## 2. Contract functions

| Function | Auth | Effect |
|---|---|---|
| `create_smart_collection(admin, name, description, rules, is_public)` | admin | Creates a smart collection, stores rule version `1`. |
| `update_smart_collection_rules(admin, collection_id, rules)` | admin | Replaces the rule set, returns the new version. |
| `rollback_smart_collection_rules(admin, collection_id, version)` | admin | Re-activates a stored rule set as a new version. |
| `refresh_smart_collection(caller, collection_id)` | any caller | Recomputes the membership of one collection. |
| `run_scheduled_curation_refresh(caller, max_collections)` | any caller | Background sweep: refreshes every collection that is due. |
| `get_smart_collection(collection_id)` | none (read) | The collection including its active rules. |
| `get_smart_collection_rules(collection_id)` | none (read) | The active rule set. |
| `list_smart_collection_rule_versions(collection_id)` | none (read) | All stored rule-set versions. |
| `list_smart_collections(start_index, limit)` | none (read) | All smart collections. |
| `list_smart_collection_projects(collection_id)` | none (read) | The curated member list. |

---

## 3. Evaluation model

Soroban has no background workers, so curation is evaluated lazily:

* **On demand** — `refresh_smart_collection` recomputes the membership of one
  collection. It is idempotent: calling it twice produces the same result.
* **Background job** — `run_scheduled_curation_refresh` is the keeper-facing
  sweep. It walks the smart collections and refreshes the ones whose last
  refresh is older than `CURATION_REFRESH_INTERVAL_SECS` (1 hour). The number
  of collections processed per call is bounded by `max_collections`.

The scan per refresh is bounded by `MAX_CURATION_SCAN_PROJECTS` (200) so a
refresh can never exceed the transaction budget. Projects are scanned by
ascending id; a deployment with more than 200 projects should therefore be
split into several refreshes over time (or the constant raised).

### What a refresh does

1. Scans the projects and computes the set of matching project ids.
2. Adds every matching project that is not a member yet (respecting
   `MAX_PROJECTS_PER_COLLECTION`).
3. Removes the projects that **curation itself** added and that no longer
   match. Manually curated members are never removed by a refresh — the
   auto-managed subset is tracked separately in `SmartCollectionKey::AutoMembers`.
4. Records `last_refreshed_at` and emits `SMARTCOL/REFRESH` with the
   added / removed / unchanged counters.

---

## 4. Rule versioning and rollback

Every rule set is stored as an immutable snapshot:

| Storage | Content |
|---|---|
| `SmartCollectionKey::RuleVersion(collection_id, version)` | one `SmartCollectionRuleVersion` |
| `SmartCollectionKey::RuleVersionList(collection_id)` | the ordered list of stored versions |
| `SmartCollection.rules` / `SmartCollection.rule_version` | the active rule set and its version |

* `update_smart_collection_rules` stores the **new** rule set as the next
  version. The previous version is kept.
* `rollback_smart_collection_rules(collection_id, version)` re-activates a
  stored snapshot **as a new version**. The history therefore stays complete
  and auditable: a rollback is never a destructive overwrite.

Events: `SMARTCOL/CREATED`, `SMARTCOL/RULES`, `SMARTCOL/ROLLBACK`,
`SMARTCOL/REFRESH`.

---

## 5. Limits

| Limit | Value |
|---|---|
| smart collections | `MAX_SMART_COLLECTIONS` = 50 |
| rules per collection | `MAX_CURATION_RULES` = 10 |
| projects scanned per refresh | `MAX_CURATION_SCAN_PROJECTS` = 200 |
| tag length | `MAX_CURATION_TAG_LEN` = 64 bytes |
| category length | `MAX_CURATION_CATEGORY_LEN` = 64 bytes |
| maximum `min_rating` | `MAX_CURATION_RATING_SCALED` = 500 (5.00 stars) |

---

## 6. Error codes

| Error | Meaning |
|---|---|
| `SmartCollectionNotFound` (145) | no such smart collection |
| `InvalidCurationRule` (146) | a rule has no condition, or a condition value is out of range |
| `TooManyCurationRules` (147) | more than `MAX_CURATION_RULES` rules |
| `SmartCollectionRuleVersionNotFound` (148) | no such rule-set version |
| `EmptyCurationRules` (149) | a smart collection needs at least one rule |
| `SmartCollectionNameExists` (150) | the collection name is already taken |
