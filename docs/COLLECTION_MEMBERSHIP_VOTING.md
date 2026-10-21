# Collection Membership Voting

How the community votes on collection membership (issue #907): who may vote,
how the votes are counted, when a proposal is auto-applied, and which
anti-gaming rules protect the collection from abuse.

Audience: collection owners, curators, community members and indexers. Pair
this with [SMART_COLLECTION_CURATION.md](SMART_COLLECTION_CURATION.md)
(rule-based curation) and [REVIEW_MODERATION.md](REVIEW_MODERATION.md).

---

## 1. Opt-in voting

Voting is **off by default**. A collection owner (or an admin) enables it:

```
set_collection_voting(caller, collection_id, enabled, threshold_bps)
```

* `threshold_bps` is the approval ratio in basis points that triggers the
  auto-accept. The default is `7_500` (75 %).
* Only the owner of the collection or an admin may change the setting.
* `threshold_bps` must be between `1` and `10_000`
  (`InvalidVoteThreshold`).

`Collection.voting_enabled` and `Collection.vote_threshold_bps` hold the
setting, so the front-end can render the vote box without an extra call.

---

## 2. Casting a vote

```
vote_on_collection_membership(voter, collection_id, project_id, direction)
```

`direction` is `VoteDirection::Approve` (add the project) or
`VoteDirection::Reject` (keep it out / remove it). The call returns the
updated `CollectionVoteTally`:

| Field | Meaning |
|---|---|
| `approve_votes` / `reject_votes` | running counters |
| `total_votes` | `approve_votes + reject_votes` |
| `approval_bps` | `approve_votes * 10_000 / total_votes` |
| `threshold_bps` | the threshold in force for the collection |
| `outcome` | `Pending`, `Approved` or `Rejected` |
| `resolved` | whether the proposal was already applied |

Reads:

* `get_collection_membership_votes(collection_id, project_id)` — the tally.
* `get_collection_membership_vote(collection_id, project_id, voter)` — a single
  vote.
* `list_collection_voted_projects(collection_id, start, limit)` — the projects
  that received votes.

---

## 3. Auto-accept / auto-reject

A proposal is resolved in the same transaction as the vote that crosses the
threshold:

* **Accept** — `approval_bps >= threshold_bps` → the project is added to the
  collection.
* **Reject** — `rejection_bps >= threshold_bps` → the project is rejected and
  removed from the collection if it was a member.

Both events are emitted: `COLVOTE/CAST` for the vote and `COLVOTE/RESOLVED`
for the resolution.

---

## 4. Anti-gaming rules

| # | Rule | Error |
|---|---|---|
| 1 | **One vote per address** — votes are keyed by `(collection_id, project_id, voter)`; a vote cannot be cast twice or flipped. | `CollectionVoteAlreadyCast` (152) |
| 2 | **Reputation requirement** — the voter must have contributed to the registry: at least `MIN_VOTER_REVIEWS` (1) authored review, at least `MIN_VOTER_REPUTATION_POINTS` (10) reviewer points, or a verified reviewer identity. Admins always qualify. | `CollectionVoteInsufficientReputation` (153) |
| 3 | **No self-promotion** — the owner of the project being voted on cannot vote on it. | `Unauthorized` (2) |
| 4 | **Quorum** — a proposal is only resolved once at least `MIN_COLLECTION_VOTES_FOR_RESOLUTION` (3) votes have been cast, so a single enthusiastic voter can never curate a collection alone. | — |
| 5 | **Finality** — a resolved proposal cannot be voted on again. | `CollectionVoteAlreadyResolved` (155) |
| 6 | **Voting must be enabled** — the collection owner has to opt in first. | `CollectionVotingDisabled` (151) |

---

## 5. Storage

| Key | Content |
|---|---|
| `CollectionVoteKey::Vote(collection_id, project_id, voter)` | one `CollectionMembershipVote` (append-only) |
| `CollectionVoteKey::ApproveCount(collection_id, project_id)` | running approve counter |
| `CollectionVoteKey::RejectCount(collection_id, project_id)` | running reject counter |
| `CollectionVoteKey::Resolved(collection_id, project_id)` | whether the proposal was applied |
| `CollectionVoteKey::VotedProjects(collection_id)` | the projects that received votes |

The member list itself is the shared `StorageKey::CollectionProjectIds(id)`
list, so a vote-resolved membership is visible through the regular collection
endpoints (`list_collection_projects`).

---

## 6. Operator notes

* **Upgrade note (breaking).** `Collection` gained the fields
  `voting_enabled` and `vote_threshold_bps`. Collection records written
  before this change do not contain them and cannot be decoded by the new
  contract — re-create or migrate the affected collections during the
  upgrade (see [`STORAGE_MIGRATION_GUIDE.md`](./STORAGE_MIGRATION_GUIDE.md)).
  New collections are created with `voting_enabled = false` and a threshold
  of `7_500` bps.
* The vote tally is derived from the counters, so it can always be recomputed
  from the individual votes (`CollectionVoteKey::Vote`).
* If the owner changes the threshold after votes were cast, the next vote
  re-evaluates the proposal against the **new** threshold. Lowering the
  threshold can therefore resolve a proposal that was previously pending.
* The project owner may still curate manually with the regular collection
  endpoints — the vote flow only *adds* to the member list, it never fights
  the owner.
