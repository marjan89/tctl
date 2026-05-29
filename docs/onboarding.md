# Substrate Device Automation

Cross-platform device automation framework for mobile apps. Semantic agents extract the full UI tree at runtime. Declarative YAML test specs drive physical devices. Acceptance criteria generate executable tests automatically. Results aggregate into a cross-platform matrix. All properties — not screenshots — are the source of truth.

## Architecture

```
Linear ticket
     ↓
preflight (user-story.md with FR state tables)
     ↓
generate-tc.py (FR states → TC YAML, applies 14 gotcha rules)
     ↓
TC YAML specs (platform-forked steps, fixture refs, assertions)
     ↓
┌─────────────┐    ┌─────────────┐
│ ddb test     │    │ idb test     │
│ (Rust)       │    │ (Swift)      │
│ Android      │    │ iOS          │
└──────┬───────┘    └──────┬───────┘
       │                    │
       ▼                    ▼
┌─────────────┐    ┌─────────────┐
│ semantic     │    │ semantic     │
│ agent        │    │ agent        │
│ (Kotlin)     │    │ (Swift)      │
│ in-app debug │    │ in-app debug │
└──────┬───────┘    └──────┬───────┘
       │                    │
       ▼                    ▼
  physical device      physical device
       │                    │
       ▼                    ▼
  result YAML          result YAML
       │                    │
       └────────┬───────────┘
                ▼
          vdb matrix
          (pass/fail × platform)
                ▼
          vdb diff --tolerances
          (design vs implementation)
```

## Components

| Component | Language | Repo | Purpose |
|-----------|----------|------|---------|
| `ddb` | Rust | `substrate-distro/device-control-android` | Android device control + test runner |
| `idb` | Swift | `substrate-distro/device-control-ios` | iOS device control + test runner |
| `vdb` | Rust | `substrate-distro/visual-debug-bridge` | Diff, validate, render, overlay, matrix |
| `fdb` | Rust | `substrate-distro/figma-debug-bridge` | Figma → YAML via kiwi protocol |
| `wdb` | Go | `substrate-distro/device-control-web` | WebSocket capture for Figma pipeline |
| semantic-agent-android | Kotlin | `substrate-distro/semantic-agent-android` | In-app UI tree extraction (Android) |
| semantic-agent-ios | Swift | `substrate-distro/semantic-agent-ios` | In-app UI tree extraction (iOS) |
| `semantic-schema` | Rust | `substrate-distro/semantic-schema` | Shared YAML schema crate |
| `switchboard` | Rust | `substrate-distro/switchboard` | Multi-agent coordination wire |
| `gate` | Rust | `substrate-distro/workflows` | Workflow state machine enforcer |
| `generate-tc.py` | Python | per-project `catalogue/scripts/` | FR state tables → TC YAML |

All binaries install to `/opt/homebrew/bin/`.

## Per-Project Glue

To onboard YOUR app, you create three things:

### 1. Bridge classes (semantic agent integration)

**Android** — implement two interfaces in your app's debug source set:

```kotlin
// app/src/debug/kotlin/com/yourapp/AgentBridge.kt
class YourAgentAuth : AgentAuth {
    override fun isAuthenticated(): Boolean = /* your auth check */
    override fun login(email: String, password: String) = /* your login */
    override fun logout() = /* your logout */
}

class YourAgentNavigator : AgentNavigator {
    override fun createSiteIntent(id: Int): Intent = /* intent to your detail screen */
    override fun createUserIntent(id: Int): Intent = /* intent to your profile screen */
}
```

Add the semantic agent as a debug dependency:

```kotlin
// build.gradle.kts
debugImplementation(project(":semantic-agent-android"))
```

**iOS** — conform to two protocols:

```swift
class YourAgentAuth: AgentAuthProvider {
    func isAuthenticated() -> Bool { /* your auth check */ }
    func login(email: String, password: String) async throws { /* your login */ }
    func logout() { /* your logout */ }
}

class YourAgentNavigator: AgentNavigationProvider {
    func createSiteViewController(id: Int) -> UIViewController { /* your detail screen */ }
    func createUserViewController(id: Int) -> UIViewController { /* your profile screen */ }
}
```

Add via SPM: `https://github.com/marjan89/semantic-agent-ios.git`

### 2. Fixtures (test data)

```yaml
# catalogue/fixtures.yaml
test_site:
  id: 12345
  name: "Your Test Location"

test_user:
  id: 67890
  name: "testuser"
  email: "test@yourapp.com"
  password: "testpass123"

empty_site:
  id: 99999
  name: "Empty Location"
```

### 3. Navigation YAML (optional, for complex navigation)

```yaml
# catalogue/navigation.yaml
app:
  package: com.yourapp.debug
  main_activity: .ui.MainActivity
  permissions:
    - android.permission.ACCESS_FINE_LOCATION
    - android.permission.ACCESS_COARSE_LOCATION
```

## Onboarding: Zero to Running TCs

### Android

```bash
# 1. Add semantic agent to your app (debug only)
# Copy semantic-agent-android/ into your project, add as debug dependency

# 2. Build and install debug APK
nosandbox ./gradlew assembleDebug
adb install app/build/outputs/apk/debug/app-debug.apk

# 3. Verify agent is running
curl http://localhost:9876/health   # should return {"status":"ok"}

# 4. Dump the UI tree
ddb ui                              # shows current screen elements

# 5. Create your first TC
cat > catalogue/tests/my-first-tc.yaml << 'EOF'
id: TC-001
name: "Home screen loads"
precondition:
  activity: MainActivity
steps:
  - action: wait_idle
    seconds: 5
  - assert: element_exists
    target: {content_fuzzy: "search"}
EOF

# 6. Run it
ddb test -d <your-device> catalogue/tests/my-first-tc.yaml

# 7. Check results
cat catalogue/tests/results/TC-001-android-*.yaml
```

### iOS

```bash
# 1. Add semantic agent via SPM (debug only)
# Add package, conform to protocols, register in AppDelegate #if DEBUG

# 2. Build and run on device/simulator
nosandbox xcodebuild -scheme YourApp -configuration Debug -destination 'id=<device>'

# 3. Verify agent
curl http://localhost:9877/health

# 4. Dump UI tree
idb ui

# 5. Create TC (same YAML as Android — cross-platform)
# 6. Run: idb test -d <device> catalogue/tests/my-first-tc.yaml
# 7. Check: catalogue/tests/results/TC-001-ios-*.yaml
```

## Runner Action Vocabulary

| Action | Parameters | What it does |
|--------|-----------|-------------|
| `tap` | `target: {content_fuzzy: "text"}` | Tap element matching text |
| `type` | `text: "input text"` | Type into focused field |
| `scroll_to` | `target: {content_fuzzy: "text"}` | Scroll until element visible |
| `wait` | `seconds: N` | Fixed wait |
| `wait_idle` | `seconds: N` | Wait for UI to settle (SSE-based) |
| `wait_event` | `text: "event_type", seconds: N` | Wait for SSE event |
| `back` | — | Press back |
| `home` | — | Press home |
| `long_press` | `target: {content_fuzzy: "text"}` | Long press element |
| `capture` | `output: path` | Dump semantic YAML |
| `capture_screenshot` | `output: path` | Screenshot to file |
| `navigate_to_site` | `site_id: N, platform: {android: [...], ios: [...]}` | Platform-forked site navigation |
| `assert element_exists` | `target: {content_fuzzy: "text"}` | Verify element on screen |
| `api_call` | `method, url, headers, body, save_as` | HTTP API call (seed/cleanup) |

### Target Matching

```yaml
{id: "elementId"}                              # exact resource ID
{text: "exact text"}                           # exact content match
{content_fuzzy: "partial"}                     # case-insensitive contains (preferred)
{content_fuzzy: "text", exclude_type: input}   # exclude input fields from match
```

## TC YAML Format

```yaml
id: TC-{number}
name: "Short description (under 60 chars)"
precondition:
  activity: MainActivity
  logged_in: true          # false = app reset + permission re-grant
steps:
  - action: navigate_to_site
    site_id: {{fixtures.test_site.id}}
    platform:
      android:
        - action: tap
          target: {content_fuzzy: "search"}
        - action: wait
          seconds: 1
        - action: type
          text: "{{fixtures.test_site.name}}"
        - action: wait
          seconds: 2
        - action: tap
          target: {content_fuzzy: "{{fixtures.test_site.name}}", exclude_type: input}
        - action: wait
          seconds: 3
      ios:
        - action: tap
          target: {content_fuzzy: "search"}
        - action: wait
          seconds: 1
        - action: type
          text: "{{fixtures.test_site.name}}"
        - action: wait
          seconds: 2
        - action: tap
          target: {content_fuzzy: "{{fixtures.test_site.name}}", exclude_type: input}
        - action: wait
          seconds: 3
  - action: wait_idle
    seconds: 5
  - action: scroll_to
    target: {content_fuzzy: "target section"}
  - assert: element_exists
    target: {content_fuzzy: "expected element"}
```

## TC YAML Complete Schema Reference

### Actions

| Action | Required fields | Optional fields | Notes |
|--------|----------------|-----------------|-------|
| `tap` | `target` | — | Retries 3x with 1s delay. Semantic + uiautomator fallback |
| `type` | `text` | — | Types into currently focused field |
| `long_press` | `target` | — | Android: triggers context menu. iOS: use `tap` on delete button instead |
| `scroll_to` | `target` | — | Scrolls down iteratively until target visible. 10 scroll attempts max |
| `scroll` | `direction` | — | Scroll up/down/left/right one viewport |
| `wait` | `seconds` | — | Fixed delay |
| `wait_idle` | `seconds` | — | SSE-based idle detection with polling fallback. Timeout = seconds |
| `wait_event` | `text`, `seconds` | — | SSE event subscription. `text` = event type name |
| `back` | — | — | Android: back button. iOS: navigation back |
| `home` | — | — | Android: home button. iOS: home button |
| `capture` | `output` | — | Dump semantic YAML to file. Supports `{platform}` variable |
| `capture_screenshot` | `output` | — | Screenshot to PNG file |
| `navigate_to_site` | `site_id`, `platform` | — | Must include android + ios step blocks |
| `api_call` | `method`, `url` | `headers`, `body`, `save_as` | For seed/cleanup. Supports `{{var}}` interpolation |

### Assert types

| Assert | Required fields | Notes |
|--------|----------------|-------|
| `element_exists` | `target` | Polls 10x at 1s intervals (semantic + uiautomator) |

### Target fields

| Field | Type | When to use |
|-------|------|------------|
| `id` | string | Exact resource ID match. Platform-specific |
| `text` | string | Exact content match. Avoid cross-platform |
| `content_fuzzy` | string | Case-insensitive substring. Preferred for all cross-platform TCs |
| `exclude_type` | string | Exclude elements of this type from matching. Use `input` for search results |

### Platform fork block

```yaml
- platform:
    android:
      - action: long_press
        target: {content_fuzzy: "question"}
    ios:
      - action: tap
        target: {content_fuzzy: "remove"}
```

### Template variables

| Pattern | Resolved from | Example |
|---------|--------------|---------|
| `{{fixtures.test_site.name}}` | `catalogue/fixtures.yaml` | "Sandhammaren" |
| `{{fixtures.test_user.email}}` | `catalogue/fixtures.yaml` | "test@app.com" |
| `{{auth_response.access_token}}` | `save_as` from prior `api_call` | Bearer token |
| `{platform}` | Runtime detection | "android" or "ios" |

### Precondition fields

| Field | Type | Default | Effect |
|-------|------|---------|--------|
| `activity` | string | required | Verify app is on this screen before starting |
| `logged_in` | bool | `true` | `false` = pm clear (Android) / reinstall (iOS) + permission re-grant |

## Per-Project Config Templates

### fixtures.yaml (required)

```yaml
# Test data your app needs. Every TC references these — never hardcode values.
test_site:
  id: 12345                    # primary test location
  name: "Your Test Location"
  full_name: "Your Test Location, Category"

empty_site:
  id: 99999                    # location with zero content
  name: "Empty Location"

test_user:
  id: 67890
  name: "testuser"
  email: "test@yourapp.com"
  password: "testpass123"

other_user:
  id: 11111                    # user with different state (e.g. no content)
  name: "Other User"

test_content:
  text: "Test content created by automation"
  author: "testuser"
```

### navigation.yaml (optional)

```yaml
app:
  package: com.yourapp.debug            # Android package name
  bundle_id: com.yourapp.debug          # iOS bundle ID
  main_activity: .ui.MainActivity       # Android entry point
  permissions:                          # auto-granted via pm grant / simctl privacy
    - android.permission.ACCESS_FINE_LOCATION
    - android.permission.ACCESS_COARSE_LOCATION
    - android.permission.CAMERA
```

### Bridge class templates

**Android** — `app/src/debug/kotlin/.../AgentBridge.kt`:
```kotlin
class YourAgentAuth : AgentAuth {
    override fun isAuthenticated(): Boolean { /* check your auth state */ }
    override fun login(email: String, password: String) { /* your login flow */ }
    override fun logout() { /* your logout flow */ }
    override fun resetState() { /* clear tokens, caches */ }
    override fun deleteAccount() { /* optional */ }
}

class YourAgentNavigator : AgentNavigator {
    override fun createSiteIntent(id: Int): Intent { /* detail screen intent */ }
    override fun createUserIntent(id: Int): Intent { /* profile screen intent */ }
}
```

**iOS** — `Sources/Debug/AgentBridge.swift`:
```swift
class YourAgentAuth: AgentAuthProvider {
    func isAuthenticated() -> Bool { /* check your auth state */ }
    func login(email: String, password: String) async throws { /* your login flow */ }
    func logout() { /* your logout flow */ }
    func resetState() { /* clear keychain, caches */ }
}

class YourAgentNavigator: AgentNavigationProvider {
    func createSiteViewController(id: Int) -> UIViewController { /* detail screen */ }
    func createUserViewController(id: Int) -> UIViewController { /* profile screen */ }
}
```

## TC File Naming Conventions

| Pattern | When | Example |
|---------|------|---------|
| `fr{N}_s{M}.yaml` | Generated from FR state table | `fr6_s1.yaml` |
| `qa-{feature}.yaml` | Hand-authored functional TC | `qa-post-happy-path.yaml` |
| `qa-000-seed-data.yaml` | API seed (runs first) | Always `000` prefix |
| `qa-999-cleanup.yaml` | API cleanup (runs last) | Always `999` prefix |
| `qa-visual-{check}.yaml` | Visual parity TC | `qa-visual-typography.yaml` |
| `qa-journey-{flow}.yaml` | Multi-screen journey | `qa-journey-full.yaml` |
| `qa-suite.yaml` | Suite definition (run_order) | One per feature area |

Result files: `{tc_id}-{platform}-{timestamp}.yaml` (auto-generated by runner).

## Gotcha Rules

See [tc-authoring-guide.md](../catalogue/tc-authoring-guide.md) for the full 14 rules. The critical ones:

1. **Always wait after navigation** — `wait_idle` or `wait 3s` after any screen transition
2. **scroll_to before assert** — can't assert what's not on screen
3. **content_fuzzy over exact text** — cross-platform text differs
4. **exclude_type: input on search** — search field matches before results
5. **Platform fork for destructive gestures** — Android long_press, iOS swipe/tap
6. **Never hardcode user data** — always `{{fixtures.*}}`
7. **One assertion per concept** — if #3 fails, #4 and #5 are noise
8. **logged_in:false poisons the session** — run after logged_in:true TCs

## Suite Execution

```yaml
# qa-suite.yaml
id: qa-suite
name: "Full QA Suite"
run_order:
  - qa-000-seed-data.yaml    # API seed
  - tc-home-screen.yaml      # no-login TCs first
  - tc-search.yaml
  - tc-post-flow.yaml        # login-required TCs
  - tc-delete-flow.yaml
  - qa-999-cleanup.yaml      # API cleanup
```

```bash
# Run full suite
ddb test -d a54 --suite catalogue/tests/qa-suite.yaml

# Results matrix
vdb matrix --results catalogue/tests/results/

# Diff against Figma baseline
vdb diff catalogue/android/screen/semantic.yaml \
        catalogue/figma/screen/semantic.yaml \
        --tolerances catalogue/manifest.yaml
```

## Overnight Run (target architecture)

```bash
# Kick before bed
ddb test -d a54 --suite qa-full-suite.yaml &
idb test -d dev-ios-one --suite qa-full-suite.yaml &

# Both runners emit per-step progress to switchboard
# Both auto-diff captures against Figma baselines
# Both emit structured result YAMLs

# Wake up to:
vdb matrix --results catalogue/tests/results/
# TC-001  android:PASS  ios:PASS
# TC-002  android:PASS  ios:FAIL(step7:element_not_found)
# TC-003  android:PASS  ios:PASS
# ...
```
