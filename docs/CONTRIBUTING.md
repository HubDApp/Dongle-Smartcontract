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
