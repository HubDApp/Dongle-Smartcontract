# Off-Chain Search Indexer Implementation Guide

This guide describes how to implement an off-chain full-text search indexer for Dongle projects (closes #909).

While Dongle provides in-contract search via `search_projects(query, start_index, limit)` and `search_projects_with_limit(query, limit)` with wildcard and phrase matching, on-chain execution is bounded by Soroban compute and storage limits. For large-scale production search experiences with typo tolerance, complex Boolean queries, and multi-field scoring, an off-chain search engine (e.g. Meilisearch, Elasticsearch, or PostgreSQL Full-Text Search) should be paired with the smart contract.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Event Ingestion & Change Data Capture (CDC)](#event-ingestion--change-data-capture-cdc)
3. [Search Index Schema](#search-index-schema)
4. [Tokenization, Normalization & Wildcards](#tokenization-normalization--wildcards)
5. [Ranking & Scoring Model](#ranking--scoring-model)
6. [Reference Pipeline Implementation](#reference-pipeline-implementation)
7. [On-Chain Fallback Pattern](#on-chain-fallback-pattern)

---

## Architecture Overview

```
                      ┌────────────────────────┐
                      │ Stellar / Soroban RPC  │
                      └───────────┬────────────┘
                                  │ Poll / Streaming Events
                                  ▼
                      ┌────────────────────────┐
                      │ Search Indexer Worker  │
                      └───────────┬────────────┘
                                  │
                 ┌────────────────┴────────────────┐
                 ▼                                 ▼
       ┌───────────────────┐             ┌───────────────────┐
       │ Inverted Index DB │             │ Metadata / Cache  │
       │ (Meilisearch/ES)  │             │ (Redis / PG)      │
       └─────────┬─────────┘             └───────────────────┘
                 │
                 ▼ Query & Ranking
       ┌───────────────────┐
       │ Frontend / API    │
       └───────────────────┘
```

1. **Indexer Worker** subscribes to contract events emitted by Dongle.
2. Changes to projects (registration, updates, reviews, endorsements, archive status) update the search document.
3. Search queries are served directly from the search index with sub-50ms latency.
4. If the off-chain indexer is unavailable, frontends can fall back to the contract's `search_projects` endpoint.

---

## Event Ingestion & Change Data Capture (CDC)

Listen to the following Soroban events from the contract:

| Event Topic | Description | Action in Search Index |
|-------------|-------------|------------------------|
| `PROJECT / REGISTERED` | New project created | Insert document into index |
| `PROJECT / UPDATED` | Project metadata edited | Update name, description, category, tags, website |
| `PROJECT / ARCHIVED` | Project owner archived project | Set `is_archived: true` (hide from default search) |
| `PROJECT / REACTIVATED` | Project unarchived | Set `is_archived: false` |
| `REVIEW / SUBMITTED` | Review submitted | Recalculate average rating & review count |
| `SUBSCRIPTION / FOLLOWED` | User followed project | Increment follower count |
| `SUBSCRIPTION / UNFOLLOWED`| User unfollowed project | Decrement follower count |
| `QUARANTINE / SET` | Project quarantined | Set `is_quarantined: true` (hide from search) |

---

## Search Index Schema

Recommended JSON document schema for each indexed project:

```json
{
  "id": 42,
  "name": "Orbital Network",
  "slug": "orbital-network",
  "description": "Cross-chain liquidity bridge and automated settlement protocol.",
  "category": "DeFi",
  "tags": ["dex", "bridge", "liquidity", "cross-chain"],
  "owner": "GA2C5RFPE6GCKMY3ZBDW7AZAOM4S26APN3",
  "verification_status": "Verified",
  "review_count": 85,
  "average_rating": 480,
  "follower_count": 120,
  "is_archived": false,
  "is_quarantined": false,
  "created_at": 1729000000,
  "updated_at": 1729100000
}
```

---

## Tokenization, Normalization & Wildcards

To provide parity with the contract's on-chain search capabilities:

1. **Case Normalization:** Convert all searchable text (names, descriptions, tags, categories) to lowercase UTF-8.
2. **Wildcard & Prefix Queries:**
   - Index n-grams (bigrams and trigrams) or use prefix trie indexing to support `*term*`, `term*`, and `*term`.
   - On-chain wildcard patterns using `*` (such as `*pay*` or `dex*`) map directly to wildcard queries in Elasticsearch or prefix search in Meilisearch.
3. **Phrase Search:**
   - Query strings enclosed in double quotes (e.g. `"automated market maker"`) must match exact sequential token adjacency without token reordering.

---

## Ranking & Scoring Model

Rank projects matching search queries by blending text relevance and on-chain authority:

$$\text{Final Score} = \text{Text Relevance (BM25)} \times W_{\text{text}} + \text{On-Chain Boost}$$

Where **On-Chain Boost** combines:
- **Rating Bonus:** $\frac{\text{average\_rating}}{10}$ (max 50 points)
- **Verification Bonus:**
  - `Verified`: +100 points
  - `Pending`: +20 points
  - `Probationary`: +10 points
  - `Unverified`: +0 points
- **Engagement Bonus:** $\log(1 + \text{review\_count} \times 2 + \text{follower\_count})$

---

## Reference Pipeline Implementation

Example TypeScript indexer handler using Stellar SDK:

```typescript
import { rpc } from "@stellar/stellar-sdk";
import { MeiliSearch } from "meilisearch";

const client = new MeiliSearch({ host: "http://127.0.0.1:7700", apiKey: "masterKey" });
const index = client.index("projects");

async function handleEvent(event: rpc.Api.GetEventsResponseEntry) {
  const topic = event.topic.map((t) => t.toString());
  if (topic[0] === "PROJECT" && topic[1] === "REGISTERED") {
    const projectId = event.topic[2];
    const projectData = await fetchProjectFromContract(projectId);
    await index.addDocuments([
      {
        id: projectData.id,
        name: projectData.name,
        slug: projectData.slug,
        description: projectData.description,
        category: projectData.category,
        tags: projectData.tags ?? [],
        verification_status: projectData.verification_status,
        is_archived: false,
        created_at: Number(projectData.created_at),
      },
    ]);
  }
}
```

---

## On-Chain Fallback Pattern

If the off-chain search indexer becomes unreachable, clients should automatically degrade gracefully to the contract endpoint:

```typescript
async function search(query: string, limit: number = 20): Promise<Project[]> {
  try {
    const searchResponse = await searchIndex.search(query, { limit });
    return searchResponse.hits;
  } catch (err) {
    console.warn("Off-chain search unavailable, falling back to Dongle contract", err);
    return await dongleContractClient.search_projects_with_limit({
      query,
      limit,
    });
  }
}
```
