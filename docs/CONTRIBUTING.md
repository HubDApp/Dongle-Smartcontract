# Contributing Guidelines

Thank you for contributing to the Dongle Smart Contract project. This document outlines the process for submitting contributions, creating Pull Requests (PRs), running local tests, and adhering to development standards.

---

## 1. Development & Branch Strategy

- **Base Branch**: `main`
- **Feature Branches**: Use descriptive branch names prefixed by category, e.g.:
  - `feature/<feature-name>` for new features
  - `fix/<bug-description>` for bug fixes
  - `docs/<doc-update>` for documentation changes

Before starting work, ensure your branch is up to date with `main`:
```bash
git fetch origin
git checkout main
git pull origin main
git checkout -b feature/your-feature-name
```

---

## 2. Local Testing & Building

### Running Tests
Always run the test suite locally before pushing changes:
```bash
# Run all tests
cargo test

# Run tests for a specific module or feature
cargo test <module_name>

# Example: test project slug logic
cargo test slug
```

### Building the Contract
Ensure the Soroban smart contract compiles cleanly to WebAssembly:
```bash
cd dongle-smartcontract
cargo build --target wasm32-unknown-unknown --release
```

### Writing & Maintenance of Test Fixtures
- When adding new fields or parameters to core contract structs (e.g. `ProjectRegistrationParams`), update the test helpers in `src/tests/fixtures.rs` to maintain test suite compatibility.
- Ensure Soroban environment compatibility in tests (e.g., using standard arrays or Soroban `Vec` as required by `no_std` / test environment constraints).

---

## 3. Pull Request Submission Guidelines

### PR Checklist
Before opening a Pull Request, verify that:
- [ ] Code compiles cleanly without errors or unexpected warnings.
- [ ] All unit and integration tests pass (`cargo test`).
- [ ] Target WASM contract builds successfully.
- [ ] Existing API contracts and backward compatibility are maintained.
- [ ] Clear documentation is provided for any new functions, events, or storage keys.
- [ ] A `CHANGELOG.md` entry was added under `## [Unreleased]` (see [section 6](#6-changelog-entries)) and `python3 scripts/validate_changelog.py` passes.

### PR Description Template
When creating a PR, provide a structured description following this template:

```markdown
## Summary
[Brief description of the changes introduced by this PR]

## Type of Change
- [ ] Bug fix (non-breaking change fixing an issue)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Breaking change (fix or feature causing existing functionality to change)
- [ ] Refactoring / Documentation update

## Changes Made
- List specific changes, modified files, added endpoints/events/storage keys

## Acceptance Criteria
- [ ] Acceptance criterion 1
- [ ] Acceptance criterion 2

## Test Coverage
- Description of tests added or updated
- Commands executed to verify changes: `cargo test <module>`

## Changelog
- [ ] Entry added to `CHANGELOG.md` under `## [Unreleased]` (category: Added / Changed / Deprecated / Removed / Fixed / Security)

## Related Issues
Closes #[issue_number]
```

---

## 4. Creating PRs

You can create PRs using GitHub CLI or the GitHub Web interface:

### Using GitHub CLI (`gh`)
```bash
gh pr create \
  --title "feat: brief description of feature" \
  --body-file pr_description.md \
  --base main \
  --head feature/your-feature-name
```

### Using GitHub Web Interface
1. Push your branch to GitHub:
   ```bash
   git push -u origin feature/your-feature-name
   ```
2. Navigate to the repository on GitHub and click **Compare & pull request**.
3. Fill in the title and description using the template above, then click **Create pull request**.

---

## 5. Troubleshooting & CI Failures

- **Merge Conflicts / Outdated Branch**:
  Rebase your feature branch against latest `main`:
  ```bash
  git fetch origin
  git rebase origin/main
  git push -f origin feature/your-feature-name
  ```
- **CI Test Failures**:
  If CI checks fail, reproduce locally via `cargo test`, apply necessary fixes, commit, and push updates to your feature branch.

---

## 6. Changelog Entries

This repository tracks version history in [`CHANGELOG.md`](../CHANGELOG.md), which
follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### Policy

- **Every user-visible change requires a changelog entry in the same PR.**
  Purely internal changes (test-only refactors, CI tweaks with no operator
  impact, typo fixes) may be skipped.
- Add your bullet under `## [Unreleased]`, inside one of the six allowed
  categories: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`.
- Create the category heading only if it does not already exist in
  `## [Unreleased]`.
- Write one line per change, in the imperative/descriptive past style used by
  existing entries, and reference the issue or PR number: `(#123)`.
- Prefix any change to an on-chain interface (function signature, storage key,
  event topic, error code) with **BREAKING** and describe the required operator
  or integrator action.

### Example

```markdown
## [Unreleased]

### Added

- `get_project_endorsements` view returning paginated endorsers (#412).

### Changed

- **BREAKING:** `list_projects` now takes `start_index` instead of `offset` (#351).
```

### Validation

The changelog structure is machine-checked. Run the validator before pushing:

```bash
python3 scripts/validate_changelog.py      # validate CHANGELOG.md
python3 scripts/test_validate_changelog.py # self-tests for the validator
```

CI runs both in the `validate-changelog` job; a malformed changelog fails the
build. The validator enforces:

1. `# Changelog` title plus Keep a Changelog and SemVer references.
2. An `## [Unreleased]` section placed above all releases.
3. Release headings of the form `## [X.Y.Z] - YYYY-MM-DD` with valid SemVer and
   ISO-8601 dates, unique and ordered newest first.
4. Only the six Keep a Changelog categories, and no empty categories.
5. A link reference definition at the bottom for every version (and no orphans).
6. Agreement between the newest released version and the `dongle-contract`
   crate version in `dongle-smartcontract/Cargo.toml`.

### Cutting a Release

1. Rename `## [Unreleased]` content into a new
   `## [X.Y.Z] - YYYY-MM-DD` section and re-add an empty `## [Unreleased]`
   heading with no categories removed from history.
2. Bump `version` in `dongle-smartcontract/Cargo.toml` (and refresh
   `Cargo.lock`) to the same `X.Y.Z`.
3. Add the `[X.Y.Z]` compare link at the bottom and repoint `[Unreleased]` to
   `compare/vX.Y.Z...HEAD`.
4. Run `python3 scripts/validate_changelog.py`, then tag the release `vX.Y.Z`.
