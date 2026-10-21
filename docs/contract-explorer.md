# Contract Explorer (#942)

Web-based explorer for live contract state: projects, reviews, admins,
per-key storage usage, search/filter, recent events, and WASM state visualization.

## Run

```sh
# serve the static UI (no build step)
python3 -m http.server -d docs 8000
# open http://localhost:8000/contract-explorer.html
```

Configure the RPC URL + contract ID at the top of the page. Reads use
`soroban-rpc` `getContractData` / `getEvents`; no secrets required.

## Features

- Live tables for projects, reviews, admins (auto-refresh, real-time polling)
- Storage usage per key (`docs/contract-explorer.html` ledger-entry size estimates)
- Filtering + search across all tables
- Recent events and transactions feed
- WASM state visualization (canvas: entries grouped by storage key prefix)
