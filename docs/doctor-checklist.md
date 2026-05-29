# tctl doctor checklist

Each check runs per device defined in project.yaml. All checks must PASS before `tctl run`.

## Checks

### 1. device_connected
**What:** Device is connected and visible to the runner.
**How:**
- Android: `ddb devices` lists the device name
- iOS: `idb devices` lists the device name
**Pass:** Device name appears in output
**Fail:** "device 'X' not found" — check USB, unlock phone, enable Developer Mode

### 2. wda_ready (iOS only)
**What:** WebDriverAgent is running on the iOS device.
**How:** `idb wda status <device>`
**Pass:** "READY at http://..."
**Fail:** "NOT RESPONDING" — run `idb wda start <device>` or `idb wda build <device> --clean --start`

### 3. agent_health
**What:** Semantic agent inside the app responds to HTTP.
**How:** `curl -s --connect-timeout 2 --max-time 5 http://<host>:<agent_port>/health`
**Pass:** Response contains `"status":"ok"`
**Fail:** Agent not started — launch the app first. Check agent_port matches project.yaml.

### 4. idle_resources
**What:** Agent reports idle resource registry (Phase 5 feature).
**How:** `curl -s http://<host>:<agent_port>/idle-resources`
**Pass:** Response contains resource names (navigation, animation, spinner, layout)
**Fail:** Agent is pre-Phase 5 build — rebuild app with IdleResourceRegistry

### 5. app_installed
**What:** Target app (by package/bundle ID) is installed on device.
**How:**
- Android: `ddb app active -d <device>` or `ddb adb -d <device> shell pm list packages | grep <package>`
- iOS: `idb app launch -d <device> <bundle_id>` (launch test)
**Pass:** App launches or package listed
**Fail:** Install the app — `ddb app install` or Xcode install

### 6. credentials
**What:** Test account credentials are set in environment.
**How:** Check `$IDB_TEST_EMAIL` and `$IDB_TEST_PASSWORD` (or `$DDB_TEST_EMAIL`/`$DDB_TEST_PASSWORD`)
**Pass:** Both vars non-empty
**Fail:** "credentials not set" — export the env vars

### 7. fixtures
**What:** fixtures.yaml exists at the path in project.yaml and parses correctly.
**How:** Read file, parse YAML, verify at least one fixture key
**Pass:** "loaded N fixtures from <path>"
**Fail:** File missing or parse error

### 8. runner_version
**What:** Runner binary hash matches expected (prevents stale binary issues).
**How:**
- Android: `ddb --version` or hash check
- iOS: `idb --version` or hash check
**Pass:** Hash matches or version reported
**Fail:** Rebuild runner from source

### 9. battery_saver (Android only)
**What:** Battery saver is OFF — throttles CPU, kills background processes.
**How:** `ddb adb -d <device> shell settings get global low_power`
**Pass:** Returns 0
**Fail:** Returns 1 — `ddb adb -d <device> shell settings put global low_power 0`

### 10. animations_disabled
**What:** Animations are disabled for faster test execution.
**How:** `curl -s -X POST http://<host>:<agent_port>/animations -d '{"enabled":false}'`
**Pass:** Response confirms animations disabled
**Fail:** Agent didn't respond — check agent_health first

## Exit codes

- `0` — all checks pass
- `1` — one or more checks failed (details on stderr)
- `2` — project.yaml not found or invalid
