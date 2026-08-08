# Verification Evidence CID Schema

Verification evidence CIDs should point to structured JSON documents that an
admin, frontend, or indexer can review consistently.

## Files

- Schema: [`../verification-evidence.schema.json`](../verification-evidence.schema.json)
- Example: [`../verification-evidence.example.json`](../verification-evidence.example.json)

## Required Content

Each evidence document must include:

- `version`: schema version using semver.
- `project`: project ID/name plus optional slug, repository, and website links.
- `submittedAt`: ISO-8601 timestamp for when the evidence was prepared.
- `summary`: plain-language statement of what the evidence proves.
- `proofs`: one or more proof items.
- `privacy`: disclosure and redaction notes.

Proof items can represent repositories, deployments, transactions, screenshots,
audits, attestations, signatures, documentation, or other verifiable material.
Use HTTPS links for mutable web pages, `ipfs://` links for pinned files, and
Stellar explorer or `stellar:` references for on-chain artifacts.

## Attestations And Signatures

Use `attestations` for signed or named statements from maintainers, auditors,
ecosystem partners, or other trusted reviewers. Use `signatures` for detached
cryptographic signatures over either the canonical JSON evidence document or a
specific proof payload.

When signing evidence, the signer should state what was signed in `message`.
Indexers should not assume every evidence document has signatures, but they can
surface signature presence as a stronger verification signal.

## Privacy And Safety

Evidence documents are intended to be public. Do not include:

- private keys, seed phrases, API keys, session tokens, or credentials;
- private user records, non-public contact details, or personal documents;
- unredacted vulnerability details that would put users or funds at risk;
- screenshots containing private dashboards, access tokens, or user data.

If proof requires sensitive context, publish a redacted summary and store the
private evidence through a separate review process. Set
`privacy.containsPersonalData` accurately and explain any redactions in
`privacy.redactionNotes`.

## Publishing Flow

1. Build the evidence document using the schema.
2. Validate the JSON locally.
3. Pin the document and any screenshot/proof artifacts to IPFS.
4. Submit the evidence document CID in the verification request.
5. Keep linked proof pages stable so future reviewers can re-check the claim.
