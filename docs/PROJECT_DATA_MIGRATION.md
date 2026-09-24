# Project Data Migration

This guide describes how to move project records from another registry into Dongle without changing the contract or trusting an unvalidated export.

## Files

- [Canonical schema](project-migration.schema.json)
- [Canonical JSON template](project-migration.template.json)
- [CSV template](project-migration.csv.example)
- [GitHub-style JSON template](project-migration.github.template.json)
- [Validator and batch preparer](../scripts/validate_project_migration.py)

## Canonical format

Normalize every source export to the canonical JSON shape before importing. Each project requires:

- `sourceId`: stable identifier in the source registry.
- `owner`: the Stellar account that will own the Dongle project.
- `name`, `slug`, `description`, and `category`: values passed to `register_project`.

Optional fields map as follows:

| Migration field | Contract field |
|---|---|
| `website` | `website` |
| `repository` | `repository_url` |
| `license` | `license` |
| `logoCid` | `logo_cid` |
| `metadataCid` | `metadata_cid` |
| `tags` | `tags` |
| `socialLinks` | `social_links` |
| `launchTimestamp` | `launch_timestamp` |
| `bountyUrl` | `bounty_url` |

The source registry's owner identity cannot be converted automatically into a Stellar account. Every imported record must contain an explicit `owner` account and should be approved by that account before submission.

## Validation and bulk preparation

The preparer validates the schema version, required fields, Stellar account shape, slug format, duplicate source IDs and slugs, URL fields, tag limits, and contract field limits. It produces bounded batches of `register_project`-compatible parameter objects while retaining `sourceId` for reconciliation.

Canonical JSON:

```sh
python3 scripts/validate_project_migration.py \
  docs/project-migration.template.json \
  --format canonical \
  --start 0 \
  --limit 100 \
  --output build/project-batch-000.json
```

CSV exports use `|` between tags and `key=url|key=url` for social links:

```sh
python3 scripts/validate_project_migration.py \
  docs/project-migration.csv.example \
  --format csv \
  --output build/project-batch-000.json
```

GitHub API-style exports must include `stellar_owner`, `dongle_category`, and `dongle_slug` because those values cannot be inferred safely:

```sh
python3 scripts/validate_project_migration.py \
  docs/project-migration.github.template.json \
  --format github \
  --output build/project-batch-000.json
```

Use `--start` and `--limit` to resume a large import. Keep the generated batch file and source ID mapping as the migration receipt. A batch must be validated successfully before any transaction is submitted.

## Submission procedure

1. Export source data and preserve the original export unchanged.
2. Map source records to the canonical template or use the CSV/GitHub adapters.
3. Assign and verify a Stellar `owner` for every project.
4. Run the preparer and inspect the generated payloads and source IDs.
5. Submit each prepared payload through `register_project`, signed by the corresponding owner and with the configured registration fee.
6. Record the returned Dongle project ID beside the source ID.
7. Reconcile counts, names, slugs, owners, and failed records before processing the next batch.

The preparer does not submit transactions or silently repair invalid data. Failed records must be corrected in the source mapping and revalidated. Since registration is one transaction per project, a failed record does not partially register another record; batches can be retried by source ID after checking the reconciliation file.

## Data handling and safety

Do not put private keys, seed phrases, API tokens, private contact data, or credentials in migration files. Treat owner addresses and source exports as sensitive operational data. Store migration receipts with restricted access and retain the original export for auditability.

## Compatibility

This migration format is off-chain and additive. It uses the current `ProjectRegistrationParams` fields and does not rewrite existing storage keys. Future field additions should bump `schemaVersion`, update the schema and preparer mapping, and document whether omitted fields default to `null` or require a new migration step.
