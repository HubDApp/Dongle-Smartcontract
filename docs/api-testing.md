# Interactive API Testing Interface (#943)

Static request-builder UI for all contract functions — no custom tooling needed.

## Run

```sh
python3 -m http.server -d docs 8000
# open http://localhost:8000/api-testing.html
```

## Features

- Request builder for all contract functions (method dropdown covers
  `get_project`, `list_projects`, `get_projects_batch`, `submit_review`,
  `get_reviews_batch`, `get_verifications_batch`, cursor pagination params, …)
- Parameter input with type validation (u64/u32/string/address/JSON)
- Request format + response preview panes
- Save/load request templates (localStorage)
- curl + JavaScript (`stellar-sdk`) snippet generation
