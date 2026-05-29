# Toolchain Production Readiness Tracker

Updated: 2026-05-30

## Phase Status

| Phase | Status |
|-------|--------|
| 0-4: Refactor | DONE — 75 tests, modules extracted, unified timeout/search/fixture |
| 5: Idle barrier | DONE — iOS 6s, Android PASS, 67/67 state machine |
| 6: Catalogue + baseline | DONE — both platforms, --capture-baseline verified |
| 7: Preflight pipeline | DONE — skill updated, schema, worked example |
| 8: tctl | DONE — doctor + run + validate working |
| 9: End-to-end pipeline | IN PROGRESS — 12/39 iOS, generator content drift. Baselines captured. Generator rewrite designed. |
| 9b: Seed hooks | DESIGNED — pre_run/post_run in project.yaml, exec action |
| 10: SPM/Maven + mount/unmount | NOT STARTED |
| 11: Docs for cold session onboarding | NOT STARTED |
| 12: Exploratory crawl skill | NOT STARTED |

## Phase 9 Blockers

1. **Generator rewrite** — Python script → tctl generate (Rust). Deterministic, per-platform baselines, tested mapping rules. DESIGNED, not implemented.
2. **Per-platform baselines** — iOS baselines captured (7 files). Android baselines NOT captured.
3. **Seeded test data** — 2 answers seeded via API. No-avatar account created but not registered. seed.sh exists but not automated via tctl.
4. **Dialog/auth timing** — wait_for chains designed but presentation idle resource too aggressive for defaults. Only via explicit wait_for.

## Pass Rates

| Platform | Pass | Runnable | Rate | Notes |
|----------|------|----------|------|-------|
| iOS | 12 | 34 | 35% | Baselines help but profile screens still use code identifiers |
| Android | 3 | 35 | 9% | Running old TCs without baselines |

## Repos (all pushed)

| Repo | GitHub | Branch |
|------|--------|--------|
| ddb | marjan89/ddb | rewrite-v3 |
| idb | marjan89/idb | feat/scroll-capture |
| switchboard | marjan89/switchboard | main |
| vdb | marjan89/vdb | main |
| semantic-agent-android | marjan89/semantic-agent-android | main + feat/idle-barrier-scroll-search |
| semantic-agent-ios | marjan89/semantic-agent-ios | main |
| tctl | marjan89/tctl | main |

## Key Decisions (ADRs)

1. Idle barrier over polling (Detox pattern)
2. Process-level subprocess kill (SIGTERM, not flag check)
3. Pre-parse fixture interpolation (raw YAML before serde)
4. Semantic agent as in-app idle resource registry
5. /scroll-search as dedicated endpoint (not on /query-when-idle)
6. wait_for causal chains (sequential idle resource queries)
7. Presentation/dialog only via explicit wait_for (not in defaults)

## Next Session

1. Iterate generator rewrite design if needed
2. Implement tctl generate (Rust) with tests
3. Implement 9b hooks (pre_run/post_run/exec)
4. Capture Android baselines
5. Register no-avatar test account
6. Rerun Phase 9 with deterministic generator + seeded data + per-platform baselines
7. Gate: >80% runnable TCs on both platforms
