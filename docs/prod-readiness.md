# Toolchain Production Readiness Tracker

Updated: 2026-05-28 21:50

## Pass Rates (latest clean runs)

| Platform | PASS | Runnable | Rate | Notes |
|----------|------|----------|------|-------|
| Android (a54) | — | — | — | Phase 5 idle barrier UNTESTED (device state issue). Hard kill works (3x improvement). Need clean app install. |
| iOS (dev-ios-one) | 37 | 44 | **84%** | Phase 5 idle barrier PROVEN: TC-19a 22s→6s (3.7x), FR6-S1 19s→6s (3.2x). |

## App Bugs (must fix in app code)

| # | Bug | TCs affected | Status |
|---|-----|-------------|--------|
| F7 | No login gate on post question/answer (Android only) | FR8-S5, FR9-S3, TC-44 | FIXED (Android). iOS has gate — TC was wrong. |
| F8 | POST question no "thank you" dialog | FR8-S3, TC-28 | INVESTIGATING — may be TC bug (incomplete flow), not app bug |
| F11 | POST answer no confirmation | FR9-S2 | iOS NOT A BUG (TC incomplete). Android TBD. |
| F12 | Action area shows on other users' profiles | TC-18a | builder-7aa6 fixing |

## TC YAML Bugs (fix in YAML, no code)

| # | Bug | TCs affected | Status |
|---|-----|-------------|--------|
| 1 | FR5 fixture interpolation ({name} not {{fixtures}}) | FR5-S1/S2/S3/S4 | claude-50b2 fixing |
| 2 | FR8-S1/FR9-S1 form text mismatch | FR8-S1, FR9-S1 | claude-50b2 fixing |
| 3 | TC-19b "see all questions" not found | TC-19b | claude-50b2 fixing |
| 4 | TC-40 "sinisa" not found on profile Q&A | TC-40 | claude-50b2 fixing |
| 5 | TC-navigate-oscar "lunda" not found | TC-navigate-oscar | claude-50b2 fixing |
| 6 | Template leak {{created_question.data.id}} in seed TC | seed | claude-50b2 fixing |
| 7 | Visual TC YAML parse errors (15 not-run) | qa-visual-* | claude-50b2 deleting (vdb inline replaces these) |
| 8 | TC-28 dialog detection on Android | TC-28 | needs investigation |

## Toolchain Gaps (runner/agent code)

| # | Gap | Impact | Effort | Status |
|---|-----|--------|--------|--------|
| 1 | ~~Step timeout granularity~~ | ~~Failing steps take 90s not 30s~~ | — | **DONE** (process-level kill -9 on every ADB call, 30s hard timeout) |
| 2 | ~~TC-level timeout termination~~ | ~~Detection works, kill doesn't interrupt ADB~~ | — | **DONE** (same fix — ADB subprocess killed at 30s) |
| 3 | vdb inline screenshots | No visual diff layer during TC runs | 2 hr | DISPATCHED |
| 4 | Baseline management | No baseline storage or diff workflow | 2 hr | NOT STARTED |
| 5 | TC generator as skill | Python script, NK-specific | 2 hr | NOT STARTED |
| 6 | Fixture probes | 3 missing test data (no-avatar, org-answer, multi-answer) | 1 hr | NOT STARTED |
| 7 | Suite-level timeout | No total suite deadline | 30 min | NOT STARTED |
| 8 | ~~Intra-iteration deadline~~ | ~~Check deadline after each ADB call~~ | — | **DONE** (process-level kill handles this) |
| 9 | Onboarding doc naming | visual-qa → substrate-device-automation | 30 min | NOT STARTED |
| 10 | TC-level hard kill | Detection fires at 120s but doesn't terminate the TC process | 1 hr | NOT STARTED |
| 11 | App lifecycle in runner | App kill/relaunch/foreground-verify between TCs — currently manual bash script | 2 hr | NOT STARTED |
| 12 | Suite circuit breaker | 3 consecutive precondition failures → abort suite with "device not responsive" | 1 hr | NOT STARTED |
| 13 | --build auto-sets hash | `ddb test --build` should set DDB_EXPECTED_HASH automatically after building | 30 min | NOT STARTED |
| 14 | Preflight formalized output | Skill produces preflight-output.yaml (schema-validated) alongside markdown | 2 hr | SCHEMA SHIPPED, skill update drafted |
| 15 | /query-when-idle endpoint | Idle-resource barrier — agent waits for idle, queries once, pushes result | 4 hr | Phase 5, ADR-004 written |
| 16 | Network idle resource | Track in-flight Retrofit/OkHttp calls in semantic agent | 2 hr | Phase 5 |
| 17 | Animation idle resource | Choreographer hook in semantic agent | 1 hr | Phase 5 |
| 18 | Scoped idle queries | Runner specifies WHICH idle resources to gate on per query | 1 hr | Phase 5 |
| 19 | Feature-organized catalogue | Restructure catalogue/ by feature (spec.yaml + baseline/ + tests/) | 2 hr | DESIGNED, not started |
| 20 | Baseline capture flag | `--capture-baseline` on both runners — writes /semantic dumps on green run | 1 hr | DESIGNED, not started |
| 21 | Preflight → spec.yaml pipeline | Preflight skill outputs formalized YAML alongside markdown | 2 hr | Schema shipped, skill update pending |
| 22 | TC generator reads spec.yaml | Generator consumes structured YAML not markdown | 1 hr | NOT STARTED |
| 23 | Git-tracked baselines | Baseline files committed to repo, git diff = change detection | 30 min | NOT STARTED |
| 24 | Test orchestrator binary (tctl) | Single `tctl run` — reads project.yaml, dispatches to ddb/idb, manages seed→test→cleanup→baseline lifecycle, aggregates results. Rust binary at substrate-distro/tctl/ | 4 hr | DESIGNED |
| 25 | tctl doctor | `tctl doctor --config project.yaml` — checks all dependencies (ddb/idb/vdb binaries, devices connected, agents responding, hash match, credentials set, fixtures exist, suite path valid). Runs before first test. | 2 hr | DESIGNED |
| 26 | Agent API proxy | POST /api/proxy on semantic agent — runner sends API calls through the agent (inside app, no sandbox). Needed for seed/cleanup TCs. | 2 hr | DESIGNED |
| 27 | tctl doctor: battery saver check | Verify battery saver OFF on all Android devices before test run. Battery saver throttles CPU and kills background processes. | 15 min | NOT STARTED |

## Docs (shipping to teammate)

| Doc | Location | Status |
|-----|----------|--------|
| Onboarding doc | substrate-distro/visual-qa/README.md | DONE (needs rename) |
| TC authoring guide | catalogue/tc-authoring-guide.md | DONE |
| Catalogue README | catalogue/README.md | DONE |
| Semantic agent spec | substrate-distro/semantic-agent-spec.md | DONE |
| Android agent README | substrate-distro/semantic-agent-android/README.md | DONE |
| iOS agent README | substrate-distro/semantic-agent-ios/README.md | DONE |
| TC YAML schema reference | in onboarding doc | DONE |

## Repos (extracted, standalone)

| Repo | Status |
|------|--------|
| substrate-distro/semantic-agent-android | DONE (3101f59) |
| substrate-distro/semantic-agent-ios | DONE (3123d2f) |
| substrate-distro/semantic-agent-spec.md | DONE |

## Milestone: "Hand to teammate"

All docs done. All repos extracted. Remaining: 9 toolchain gaps (~12 hr) + 4 app bugs + 8 YAML bugs.
