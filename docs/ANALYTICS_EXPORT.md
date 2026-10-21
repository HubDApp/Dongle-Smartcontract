# Analytics Export

Issue #934. `export_analytics_data` returns registry metrics for a timestamp
window, for BI dashboards and compliance reporting. It is a read-only
endpoint and needs no auth; all inputs are public on-chain data.

## Endpoints

```text
export_analytics_data(start: u64, end: u64, format: ExportFormat) -> AnalyticsExport
export_analytics_data_filtered(start, end, format, filter: AnalyticsFilter) -> AnalyticsExport
```

- `start` and `end` are Unix timestamps, both inclusive. `start > end` returns
  `InvalidInput`.
- `format` is `Json` or `Csv`.
- `filter` narrows the projects counted:
  - `category`: exact category match, or `None` for all.
  - `verified_only`: only projects currently `Verified`.
  - `start_project_id`, `limit`: paging (below).

## Metrics

All metrics count only projects that pass the filter.

| Metric | Definition |
|--------|------------|
| `projects_scanned` | Existing projects read in this page |
| `projects_matched` | Scanned projects that passed the filter |
| `projects_created` | `created_at` in the window |
| `reviews_created` | Reviews (hidden included) with `created_at` in the window |
| `verification_requests` | Records with `requested_at` in the window |
| `verifications_approved` / `verifications_rejected` | Records decided in the window whose status is `Verified` / `Rejected` |
| `verification_rate_bps` | approved ÷ (approved + rejected) in basis points; 0 if none decided |
| `verification_fee_revenue` | Sum of `fee_amount` for requests in the window |
| `registration_fee_revenue` | Latest registration fee payment per distinct owner, if paid in the window |
| `total_fee_revenue` | Sum of the two fee metrics |

Only the latest registration payment per address is stored on-chain, so
earlier payments by the same owner are not counted. Refunds are not
subtracted.

## Paging

One call scans at most `MAX_ANALYTICS_SCAN_SIZE` (50) project IDs. When more
remain, `next_project_id` is set; pass it as `filter.start_project_id` and add
the page metrics together off-chain (recompute `verification_rate_bps` from
the summed approved and rejected counts). `export_analytics_data` always
scans the first page.

## Payload

`payload` holds the metrics rendered in the requested format.

JSON (fee amounts are strings so `u128` values survive JSON parsers):

```json
{"start":0,"end":100,"projects_scanned":1,"projects_matched":1,"projects_created":1,
 "reviews_created":0,"verification_requests":0,"verifications_approved":0,
 "verifications_rejected":0,"verification_rate_bps":0,
 "verification_fee_revenue":"0","registration_fee_revenue":"0","total_fee_revenue":"0"}
```

CSV (header plus one row, newline-terminated):

```text
start,end,projects_scanned,projects_matched,projects_created,reviews_created,verification_requests,verifications_approved,verifications_rejected,verification_rate_bps,verification_fee_revenue,registration_fee_revenue,total_fee_revenue
0,100,1,1,1,0,0,0,0,0,0,0,0
```

The JSON payload above is wrapped for readability; the real payload is one line.
