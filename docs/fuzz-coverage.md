# Fuzz Coverage Dashboard

Property-based fuzzing harness: `dongle-smartcontract/src/tests/fuzz_test_harness.rs` (proptest).

| Target | What is fuzzed | Invariant |
| --- | --- | --- |
| `fuzz_rating_range_never_misclassifies` | Random `u32` ratings (valid + invalid) | Valid iff in `1..=5` |
| `fuzz_view_cache_ttl` | Random TTL, payload, ledger advance | Hit before expiry, evicted after |
| `fuzz_rate_limiter_accounting` | Random limits and call counts | Exactly `min(N, L)` succeed; rest `RateLimitExceeded` |
| `fuzz_trusted_bypass` | Random call counts as trusted user | Bypass unlimited; quota untouched afterwards |

- **CI:** `fuzz` job in `.github/workflows/ci.yml` runs the harness on every push/PR.
- **Regression suite:** failures persist to `dongle-smartcontract/proptest-regressions/` and re-run automatically.
- **Run locally:** `cargo test -p dongle-contract fuzz_test_harness`.
