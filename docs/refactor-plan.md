# Toolchain Refactor Plan v2

Based on 8 audits + 4 targeted code reviews + 4 industry research reports. 2026-05-29.

## Principles

1. Tests first, refactor against them
2. One phase at a time, regression run between each
3. One mechanism per concern — no parallel systems
4. Both runners (ddb + idb) refactor in parallel with mirrored APIs
5. Adopt proven industry patterns (Detox idle barrier, Appium explicit-only waits, pytest fixture scoping, Mustache interpolation)
6. Control tests from Phase 3 onward — specific TCs that exercise the refactored paths

## Data Summary

- 985 TC results analyzed, 79.6% pass rate
- #1 failure: "element not found" (92 instances) — polling timing
- ddb: 2300 lines, 3 dead functions, 6 search paths, 5 timeout mechanisms, 2 fixture systems
- idb: 1750 lines, 4 dead blocks, 5 duplicates, 3 search paths, multiple timeout strategies
- Both semantic agents: duplicate nav handlers, iOS missing SSE entirely
- Test coverage: fixture interpolation only — zero timeout, zero bounds parsing, zero search tests
- Industry: Detox idle barrier eliminates timing failures. Appium: never mix implicit+explicit waits. pytest: scoped fixture DI. Espresso: in-process IdlingResource (our agent is the equivalent).

## Phase 0: Expand Test Suite

BEFORE any refactor. Tests lock in current behavior so refactoring can't silently break it.

### ddb tests (Rust, `#[test]`):

```
test_extract_yaml_int_after_stops_at_nonmatching_line
  → verifies AND vs OR logic bug (currently broken — test SHOULD fail, fix in Phase 1)

test_extract_ui_bounds_invalid_xml_returns_error
  → input: "<node bounds=\"garbage\"/>" → expect Err, not Ok((0,0))

test_extract_ui_bounds_valid_xml_returns_center
  → input: "<node bounds=\"[0,0][100,200]\"/>" → expect Ok((50,100))

test_adb_subprocess_killed_at_30s
  → spawn `sleep 60` via adb wrapper → verify killed within 35s, returns timeout error

test_element_search_fallback_order
  → mock 3 sources, first returns None, second returns match → verify second source used

test_jaccard_threshold_boundary
  → "questions answers" vs "questions & answers" at 0.59 and 0.61 → verify threshold behavior

test_fixture_interpolation_integer_preserved
  → "{{fixtures.test_site.id}}" in raw YAML → verify parsed as i64(31255) not string

test_fixture_interpolation_missing_key_passthrough
  → "{{fixtures.nonexistent.field}}" → verify kept as literal string, no panic

test_fixture_precedence_api_over_file
  → fixtures.yaml has key=A, api save_as has key=B → verify B wins
```

### idb tests (Swift, `XCTestCase`):

```
test_filehandle_write_invalid_encoding_no_crash
  → pass non-UTF8 data → verify graceful error, no force-unwrap crash

test_idle_wait_functions_produce_same_result
  → call both waitIdle() and waitForIdle() with same input → verify same output (consolidation prerequisite)

test_tmp_files_cleaned_after_run
  → run a minimal spec → verify /tmp/idb-* files deleted

test_wda_predicate_search_no_match_returns_nil
  → search for nonexistent element → verify nil return, no crash

test_platform_fork_parser_shared_steps_preserved
  → YAML with platform fork + shared steps after → verify shared steps in output

test_fixture_interpolation_integer_fields
  → "{{fixtures.test_site.id}}" → verify integer 31255

test_api_call_save_as_available_in_next_step
  → api_call with save_as → verify ctx.vars contains the response
```

**Gate: all tests pass before Phase 1 begins.**

## Phase 1: Prune Dead Code + Fix Bugs

Zero regression risk — removing unused functions and fixing known bugs.

### ddb:
- Remove `get_density()` (~line 1998) — never called
- Remove `poll_for_element()` (~line 2043) — superseded by SSE-first
- Remove `is_page_stable()` (~line 1835) — never called
- FIX `extract_yaml_int_after` logic bug (AND → OR) — test_extract_yaml_int_after now passes
- FIX `extract_ui_bounds` silent (0,0) → return Err on invalid bounds — test_extract_ui_bounds_invalid_xml now passes

### idb:
- Remove `case "assert"` in executeAction (lines 926-944)
- Remove `lookupSiteName()` (lines 1570-1601)
- Remove `lookupUser()` (lines 1603-1628)
- Remove `scrollToTop()` (lines 1346-1355)

### Both:
- Remove any TODO/FIXME comments that reference shipped work

**Regression: all Phase 0 tests pass. `cargo test` / `swift test` green.**

## Phase 2: Consolidate Duplicates

Same logic, one copy. No behavior change.

### ddb:
- `extract_ui_bounds()` + `extract_ui_bounds_fuzzy()` → single `parse_bounds(xml, target, match_mode)` 
- `interpolate_raw()` + `RunContext::interpolate()` → single `FixtureResolver::resolve(template)` (prep for Phase 4)
- Navigation.yaml loader + fixtures.yaml loader → single `load_config(paths) → ConfigStore`

### idb:
- `waitIdle()` + `waitForIdle()` → single `waitForIdle(timeout:)`
- `tapViaWDAClick()` + `findElementViaWDA()` → single `wdaQuery(predicate:) → [WDAElement]`
- Two YAML string extractors → one `extractYAMLValue(key:from:)`
- `navigate_to_site` + `navigate_to_user` idle loops → `waitForAgentIdle(timeout:)`

### Semantic agents:
- `handleNavigateSite` + `handleNavigateUser` → `handleNavigate(type:id:)` on both platforms

**Regression: all Phase 0+1 tests pass.**

## Phase 3: Extract Mixed Concerns

Move code to proper homes. No behavior change — just file boundaries.

### ddb — new modules:
| Module | Extracted from | Contains |
|--------|---------------|----------|
| `timeout.rs` | test.rs lines ~1101, ~1421, ~1593 | TimeoutManager struct (prep for Phase 4 unification) |
| `element.rs` | test.rs lines ~1679-1802, ~2353-2385 | find_element, check_element_sources, parse_bounds |
| `fixture.rs` | test.rs lines ~411, ~882-900, ~2425-2437 | FixtureResolver, load, interpolate |
| `observability.rs` | test.rs scattered | heartbeat, switchboard send, screenshot capture |
| `adb.rs` | already exists | subprocess spawn + kill-9 timeout (already extracted) |

test.rs becomes: setup → step loop → dispatch to modules → reporting. ~500 lines, down from 2300.

### idb — new files:
| File | Extracted from | Contains |
|------|---------------|----------|
| `TimeoutPolicy.swift` | Test.swift lines ~35, ~51, ~71, ~753 | TimeoutPolicy enum, deadline tracking |
| `ElementSearcher.swift` | Test.swift lines ~1359-1475 | Protocol + WDA + agent search implementations |
| `FixtureResolver.swift` | Test.swift lines ~80-92, ~1733-1750 | Load, interpolate, scoped resolution |
| `YAMLParser.swift` | Test.swift lines ~315-501 | Step parsing (or replace with Yams dependency) |
| `AppLifecycle.swift` | Test.swift lines ~1177-1287 | enforceLoggedOut, app launch, permissions |

Test.swift becomes orchestration only. ~400 lines, down from 1750.

### Semantic agents:
- iOS: implement `/stream` SSE endpoint (parity with Android)
- iOS: implement 4 event types: `activity`, `idle`, `scroll`, `keyboard`
- Both: extract `handleNavigate` template (done in Phase 2)

### Control tests (Phase 3):

These TCs exercise the refactored code paths. Run BEFORE and AFTER Phase 3 — same results = no regression.

| Control TC | Exercises | Why this TC |
|-----------|-----------|-------------|
| TC-19a | element search (scroll_to + assert on site detail) | Exercises find_element + parse_bounds extraction |
| TC-28 | timeout path (POST wait + dialog detection) | Exercises timeout manager + element_exists fallback |
| TC-39 | full journey (14 steps, navigate + scroll + assert + back) | Exercises orchestration layer (the code that stays in test.rs) |
| TC-35 | platform fork + dialog (long_press → assert delete → tap cancel) | Exercises element search + platform fork parser |
| FR6-S1 | fixture interpolation (navigate_to_site with fixture ref) | Exercises FixtureResolver |
| FR8-S4 | scroll_to + tap + assert (post question validation) | Exercises element.rs + timeout.rs interaction |

**Run control TCs BEFORE extraction. Record results. Run AFTER extraction. Diff must be zero.**

**Regression: all Phase 0+1+2 tests pass + control TCs unchanged.**

## Phase 4: Unify Systems

Single mechanism per concern. This is where behavior changes. Industry patterns applied.

### 4A: Timeout Unification

**Pattern: Appium explicit-only waits. Never compound timeouts.**

```rust
// timeout.rs (ddb) / TimeoutPolicy.swift (idb)
pub struct TimeoutManager {
    tc_deadline: Instant,        // outermost — total TC time (120-300s)
    step_deadline: Instant,      // per step (default 30s)
    
    pub fn check(&self) -> Result<(), TimeoutLevel> // called in every loop
    pub fn arm_subprocess(pid, secs) -> Guard       // kill-9 on drop
    pub fn reset_step(&mut self, secs)              // called between steps
    pub fn remaining(&self) -> Duration             // for subprocess timeouts
}
```

- Replaces: 5 ddb mechanisms, 4 idb mechanisms
- `arm_subprocess` returns a Guard — drop kills the child (RAII pattern)
- `check()` returns Err(TimeoutLevel::Step) or Err(TimeoutLevel::TC) — caller decides how to handle
- No implicit waits anywhere — every wait is explicit with a timeout from the manager
- Subprocess timeout = `min(30s, manager.remaining())` — subprocess can't outlive its step

### 4B: Element Search Unification

**Pattern: Espresso accessibility-ID-first. One entry point, configurable sources.**

```rust
// element.rs (ddb) / ElementSearcher.swift (idb)
pub fn find_element(
    target: &Target,
    sources: &[ElementSource],  // [Semantic, UIAutomator, Activity]
    timeout: &TimeoutManager,
) -> Result<SearchResult, SearchError>
```

- Single entry point for: tap, scroll_to, element_exists, precondition check, long_press
- Source priority configurable per call (scroll_to might skip Activity source)
- Each source returns `Option<SearchResult>` — first match wins
- No caching across calls — fresh data every time (Maestro pattern: never cache stale hierarchies)
- Jaccard threshold configurable but default 0.6

### 4C: Fixture Resolver Unification

**Pattern: pytest scoping + Mustache interpolation + FactoryBot precedence.**

```rust
// fixture.rs (ddb) / FixtureResolver.swift (idb)
pub struct FixtureResolver {
    layers: Vec<FixtureLayer>,  // ordered by precedence (lowest first)
}

enum FixtureLayer {
    File(HashMap<String, Value>),     // fixtures.yaml (lowest)
    Navigation(HashMap<String, Value>), // navigation.yaml
    ApiResponse(HashMap<String, Value>), // save_as (highest)
}

pub fn resolve(&self, template: &str) -> String
  // Mustache-style: {{fixtures.test_site.id}} → "31255"
  // Single syntax: {{key.field}} — no triple braces
  // Pre-parse: runs on raw YAML string before serde parse
  // Runtime: resolve() called again for api save_as refs
```

- Single `{{key.field}}` syntax everywhere — no `{{{triple}}}` vs `{{double}}`
- Precedence chain: file < navigation < api_response < TC-level override
- Pre-parse on raw YAML string (integers survive parsing)
- Runtime resolve for api_call save_as (populated mid-TC)
- Missing key → keep literal string (no panic, no silent empty)

### Control tests (Phase 4):

Same control TCs as Phase 3, plus:

| Control TC | Exercises | Why |
|-----------|-----------|-----|
| FR10-S1 | Timeout termination (scroll_to on missing element) | Verifies unified timeout kills within 30s, not 210s |
| FR1-S1 | Element search on user profile | Verifies unified search finds elements across screen types |
| FR8-S3 | POST + dialog detection + SSE | Verifies timeout + search + SSE interaction |
| TC-44 | Unauthenticated flow (logged_in: false precondition) | Verifies fixture resolver handles precondition state |

**Run ALL control TCs before Phase 4. Record PASS/FAIL + timing. After Phase 4: same PASS/FAIL, timing improved.**

**Regression: all tests pass + control TCs unchanged + FR10-S1 completes in <30s (was 210s).**

## Phase 5: Idle-Resource Barrier (Detox Architecture)

After Phases 0-4 are stable. This is the architecture shift that eliminates the #1 failure category.

**Pattern: Detox grey-box idle barrier. Turn timing problems into synchronization problems.**

### 5A: Semantic Agent — Idle Resource Registry

```kotlin
// Android SemanticServer.kt
class IdleResourceRegistry {
    val resources = mutableListOf<IdleResource>()
    
    fun register(resource: IdleResource)
    fun isAllIdle(): Boolean = resources.all { it.isIdle() }
    fun waitForIdle(timeout: Duration, callback: () -> Unit)
}

interface IdleResource {
    val name: String
    fun isIdle(): Boolean
    fun registerCallback(callback: () -> Unit)
}

// Built-in resources:
class UIThreadIdleResource     // MessageQueue.IdleHandler
class NetworkIdleResource      // Retrofit/OkHttp interceptor, count in-flight
class ScrollIdleResource       // RecyclerView.SCROLL_STATE_IDLE (already exists)
class AnimationIdleResource    // Choreographer.FrameCallback
class LayoutIdleResource       // ViewTreeObserver.OnGlobalLayoutListener (already exists)
```

Same pattern in Swift for iOS agent.

### 5B: New Endpoint — POST /query-when-idle

```
POST /query-when-idle
{
  "match": {"content_fuzzy": "questions & answers"},
  "idle_resources": ["ui_thread", "network", "scroll", "layout"],
  "timeout": 5
}

→ Response (element found after idle):
{
  "found": true,
  "element": {"x": 540, "y": 1200, "content": "Questions & Answers", ...},
  "idle_wait_ms": 230,
  "source": "view_tree"
}

→ Response (timeout):
{
  "found": false,
  "timeout": true,
  "idle_resources_status": {"network": true, "scroll": true, "layout": false}
}
```

Flow:
1. Agent receives query
2. Waits for ALL specified idle resources to report idle (event-driven, not polling)
3. Once idle: walks view tree ONCE
4. Returns match or "not found"
5. If timeout before idle: returns status of each resource (tells runner WHAT is still busy)

### 5C: Runner — Replace Polling with Barrier Query

```rust
// element.rs — new source
ElementSource::IdleBarrier {
    match_target: Target,
    idle_resources: vec!["ui_thread", "network", "scroll", "layout"],
    timeout_s: 5,
}

// find_element priority becomes:
// 1. IdleBarrier (agent waits for idle, queries once) — primary
// 2. Semantic (direct /semantic query) — fallback for agents without /query-when-idle
// 3. UIAutomator (/sdcard dump) — last resort
```

### 5D: Maestro-style Retry Envelope (safety net)

If `/query-when-idle` returns `found: false`:
1. Wait 1s
2. Retry `/query-when-idle` (idle resources may have cycled)
3. If still not found after 3 retries: FAIL

This is the Maestro pattern — implicit retry with fresh query. But because each retry goes through the idle barrier, each query is against a settled UI. Not blind polling.

### Control tests (Phase 5):

| Control TC | Before Phase 5 | Expected After |
|-----------|---------------|----------------|
| FR10-S1 (scroll_to missing element) | FAIL at 30s (timeout) | FAIL at ~5s (idle barrier timeout) |
| TC-19a (Q&A section assert) | PASS at ~15s (scroll + poll) | PASS at ~2s (idle barrier + single query) |
| TC-39 (14-step journey) | PASS at ~45s | PASS at ~15s (no poll waits between steps) |
| FR6-S1 (site detail Q&A) | PASS at ~18s | PASS at ~5s |

**Performance target: passing TCs complete 3-5x faster. Failing TCs fail in <5s, not 30-90s.**

**Regression: all tests pass + all control TCs same PASS/FAIL + timing improved.**

## Timeline Estimate (revised)

| Phase | Effort | Risk | Gate |
|-------|--------|------|------|
| 0: Tests | 4 hr | None | All tests pass |
| 1: Prune + fix bugs | 1 hr | Zero | Tests pass, bug fix tests now green |
| 2: Consolidate | 3 hr | Low | Tests pass |
| 3: Extract | 4 hr | Medium | Tests pass + control TCs unchanged |
| 4: Unify | 6 hr | Medium-High | Tests pass + control TCs + FR10-S1 <30s |
| 5: Idle barrier | 8 hr | New feature | Tests pass + control TCs + TC-19a <5s |
| **Total** | **26 hr** | **Phased** | |

## Rules

- No phase starts before the previous phase's gate passes
- No feature work during Phases 0-4 — structural changes only
- Phase 5 is a new feature — can ship independently
- Both runners refactor in parallel with mirrored APIs
- Every commit: `refactor(phase-N): description`
- Integration tests expand WITH each phase
- Control TC results recorded before AND after each phase from 3 onward
- If a control TC regresses: STOP, investigate, fix before continuing
- Builders document what they learned in each phase (feeds back to skills + docs)

## Phase 6: Feature-Organized Catalogue

After Phase 5 is stable.

- Restructure catalogue/ by feature: spec.yaml + baseline/ + tests/
- Add `--capture-baseline` flag to both runners (writes /semantic dumps on green run)
- Git-tracked baselines — git diff = change detection
- Baseline builds organically as TCs visit screens

## Phase 7: Formalized Preflight Pipeline

- Preflight skill outputs preflight-output.yaml (schema-validated) alongside markdown
- TC generator consumes structured YAML, not markdown regex
- Schema at catalogue/preflight-output-schema.yaml (shipped)

## Phase 8: Test Orchestrator (tctl)

- Rust binary at substrate-distro/tctl/
- `tctl run --config project.yaml --suite qa --devices a54,dev-ios-one`
- project.yaml: env vars, package names, agent ports, device list, suite path
- Lifecycle: seed → test → cleanup → capture baseline → aggregate matrix
- `tctl doctor`: checks all dependencies before first run
- Agent API proxy (POST /api/proxy) for sandbox-safe seed/cleanup TCs
