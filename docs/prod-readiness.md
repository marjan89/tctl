# Toolchain Production Readiness Tracker

Updated: 2026-05-29 11:00

## Pass Rates (latest clean runs)

| Platform | PASS | Runnable | Rate | Notes |
|----------|------|----------|------|-------|
| Android (a54) | — | — | — | Idle barrier works, setup sequence too slow (pm dump fix shipped, awaiting rerun) |
| iOS (dev-ios-one) | 37 | 44 | **84%** | Phase 5 idle barrier PROVEN: TC-19a 22s→6s (3.7x), FR6-S1 19s→6s (3.2x) |

## Toolchain Gaps

| # | Gap | Status |
|---|-----|--------|
| 1 | ~~Step timeout granularity~~ | **DONE** — process-level kill-9, 30s |
| 2 | ~~TC-level timeout termination~~ | **DONE** — thread-based recv_timeout + hard kill |
| 3 | vdb inline screenshots | NOT STARTED — architecture changed to capture-during-TC |
| 4 | ~~Baseline management~~ | **DONE** — --capture-baseline flag on both runners, verified iOS |
| 5 | TC generator as skill | NOT STARTED — Python script still in use |
| 6 | Fixture probes | NOT STARTED — 3 missing test data sets |
| 7 | Suite-level timeout | NOT STARTED |
| 8 | ~~Intra-iteration deadline~~ | **DONE** — process-level kill handles this |
| 9 | Onboarding doc naming | NOT STARTED — docs moved to tctl, still called visual-qa |
| 10 | ~~TC-level hard kill~~ | **DONE** — thread + recv_timeout, setup budget added |
| 11 | App lifecycle in runner | NOT STARTED — still manual bash script |
| 12 | Suite circuit breaker | NOT STARTED |
| 13 | --build auto-sets hash | NOT STARTED |
| 14 | ~~Preflight schema~~ | **DONE** — preflight-output-schema.yaml shipped |
| 15 | ~~/query-when-idle endpoint~~ | **DONE** — both platforms, 4 idle resources each |
| 16 | ~~Network idle resource~~ | **DONE** — atomic in-flight counter |
| 17 | ~~Animation idle resource~~ | **DONE** — iOS NavigationIdleResource + AnimationIdleResource |
| 18 | ~~Scoped idle queries~~ | **DONE** — idle_resources array in /query-when-idle |
| 19 | ~~Feature-organized catalogue~~ | **DONE** — features/q-and-a/ with tests/ + baseline/ + spec.yaml |
| 20 | ~~Baseline capture flag~~ | **DONE** — both runners, verified iOS |
| 21 | Preflight skill update | SPEC'D — tctl/docs/preflight-skill-update.md |
| 22 | TC generator reads spec.yaml | NOT STARTED — depends on #21 |
| 23 | Git-tracked baselines | NOT STARTED — awaiting green Android run to populate |
| 24 | tctl binary | SCAFFOLDED — repo + stubs at marjan89/tctl |
| 25 | tctl doctor | DESIGNED — battery saver check added to spec |
| 26 | Agent API proxy | DESIGNED — needed for sandbox-safe seed TCs |
| 27 | Battery saver check | NOT STARTED — tctl doctor item |
| 28 | Samsung A54 setup optimization | IN PROGRESS — pm dump removed (534ed0d), awaiting rerun |

## Summary

**DONE: 14 items** (1, 2, 4, 8, 10, 14, 15, 16, 17, 18, 19, 20 + Phase 0-5 refactor)
**IN PROGRESS: 2** (28 Samsung setup, 21 preflight skill)
**DESIGNED/SPEC'D: 4** (24 tctl, 25 doctor, 26 API proxy, 21 preflight)
**NOT STARTED: 8** (3, 5, 6, 7, 9, 11, 12, 13, 22, 23)

## Repos (all pushed)

| Repo | GitHub | Branch |
|------|--------|--------|
| ddb | marjan89/ddb | rewrite-v3 |
| idb | marjan89/idb | feat/scroll-capture |
| switchboard | marjan89/switchboard | main |
| vdb | marjan89/vdb | main |
| semantic-agent-android | marjan89/semantic-agent-android | main |
| semantic-agent-ios | marjan89/semantic-agent-ios | main |
| tctl | marjan89/tctl | main |

## Docs (all in tctl/docs/)

| Doc | Status |
|-----|--------|
| refactor-plan.md | Phases 0-8, v2 |
| prod-readiness.md | This file |
| semantic-agent-spec.md | Contract for any platform agent |
| onboarding.md | How to add a new project |
| tc-authoring-guide.md | 17 rules |
| preflight-output-schema.yaml | Formalized schema |
| preflight-skill-update.md | Phase 3b spec |
| ADR-001 through ADR-004 | 4 architectural decision records |

## Integration Tests

| Tool | Tests | Status |
|------|-------|--------|
| ddb | 30/30 | GREEN |
| idb | 45/45 | GREEN |
| semantic agent | 10/10 (1 expected fail: /query-when-idle test should now PASS) | NEEDS RERUN |
