# Feature Campaigns

Relevant source: `dongle-smartcontract/src/campaign_registry.rs`.

## Time-bound campaigns (issue #926)

A `FeatureCampaign` is an admin-curated set of projects highlighted for a
special event or theme between `start_date` (inclusive) and `end_date`
(exclusive), both Unix timestamps in seconds.

| Function | Access | Description |
| --- | --- | --- |
| `create_feature_campaign(admin, campaign_config)` | Admin | Creates a campaign from a `FeatureCampaignConfig { name, theme, start_date, end_date }` and returns its ID. |
| `add_to_campaign(admin, project_id, campaign_id)` | Admin | Adds a project. Allowed until `end_date`. |
| `get_projects_by_campaign(campaign_id, start_index, limit)` | Public | Campaign projects in insertion order, paginated. |
| `record_campaign_interaction(viewer, campaign_id, project_id, interaction)` | Viewer signs | Counts an `Impression` or `Engagement` while the campaign is active. |
| `get_campaign_stats(campaign_id)` | Public | Total impressions / engagements for the campaign. |
| `get_campaign_project_stats(campaign_id, project_id)` | Public | Impressions / engagements for one project in the campaign. |
| `get_feature_campaign`, `list_feature_campaigns`, `get_active_campaigns` | Public | Campaign lookups. |

### Validation

- `name` and `theme`: 1–64 bytes.
- `end_date` must be after `start_date` and in the future; the campaign can
  last at most 365 days (`MAX_CAMPAIGN_DURATION_SECS`).
- At most `MAX_PROJECTS_PER_CAMPAIGN` (50) projects per campaign
  (`CampaignFull`). Adding a project twice fails with `AlreadyInCollection`.
- Interactions are only accepted inside the campaign window
  (`CampaignNotActive`) and for projects in the campaign (`NotInCollection`).

### Events

| Topics | Data |
| --- | --- |
| `CAMPAIGN / CREATED / campaign_id` | `FeatureCampaignCreatedEvent` |
| `CAMPAIGN / ADDED / campaign_id` | `ProjectAddedToCampaignEvent` |
| `CAMPAIGN / INTERACT / campaign_id` | `CampaignInteractionEvent` |
