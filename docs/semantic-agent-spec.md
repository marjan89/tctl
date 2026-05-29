# Semantic Agent Specification v1

Contract for runtime UI introspection agents. Any platform agent (Android, iOS, Flutter, Compose Multiplatform, React Native) that implements this spec works with `ddb`/`idb` runners, `vdb` visual pipeline, and the TC execution framework.

## HTTP Server

Agent runs an HTTP server on the device/simulator. Default ports: Android 9876, iOS 9877. Configurable via environment variable (`DDB_AGENT_PORT` / `IDB_AGENT_PORT`).

## Endpoints

### GET /health

Returns agent status. Runner uses this to verify agent is alive before TC execution.

**Response:** `200 OK`
```json
{"status": "ok"}
```

### GET /semantic

Walk the full view tree and return YAML. This is the core endpoint — everything else is supplementary.

**Query parameters:**

| Param | Type | Default | Effect |
|-------|------|---------|--------|
| `scroll` | int | — | Scroll-capture mode: capture N scroll positions, stitch into full-page YAML |

**Response:** `200 OK`, `Content-Type: text/yaml`

```yaml
screen: MainActivity
device: Samsung Galaxy A54 (SM-A546B)
platform: android
timestamp: 2026-05-28T19:00:00Z
source: instrumented
viewport:
  width: 384
  height: 832
  density: 2.8125
elements:
- id: site_title
  platform_id: siteTitleTextView
  type: text
  content: "Stockholm Archipelago Trail"
  bounds:
    x: 19
    y: 329
    w: 344
    h: 62
  z_index: 42
  clickable: false
  enabled: true
  accessible: true
  a11y_label: "Stockholm Archipelago Trail"
  font:
    family: poppins
    weight: semibold
    size: 22
  foreground: "#08292F"
  background: "#FFFFFF"
  corner_radius: 12
  border:
    width: 1
    color: "#E0E0E0"
  gradient:
    type: linear
    colors: ["#FF0000", "#0000FF"]
    orientation: top_bottom
  line_count: 2
  truncated: false
  elevation: 4.0
  margin:
    top: 0
    bottom: 8
    start: 17
    end: 17
  image:
    resource: "ic_hiking_24"
    type: vector
  image_path: "images/ic_hiking_24.png"
```

### GET /overlay

Draw colored bounding boxes on the device screen for all elements from the last `/semantic` walk. Each element gets a deterministic color from djb2 hash of its id/content.

**Response:** `200 OK`
```json
{"status": "drawn", "count": 42}
```

### DELETE /overlay

Remove all overlay drawings from the screen.

**Response:** `200 OK`
```json
{"status": "cleared"}
```

### GET /idle

Returns whether the UI is settled — no animations, no pending layout passes, no scroll in progress.

**Response:** `200 OK`
```json
{"idle": true}
```

The runner polls this or uses `/stream` SSE for push-based idle detection.

### GET /stream

Server-Sent Events (SSE) stream. Emits real-time events as the UI changes. The runner subscribes for event-driven waits instead of polling.

**Event types:**

| Event | When | Data |
|-------|------|------|
| `activity` | Activity/screen changes (resume/pause) | `{"name": "MainActivity", "state": "resumed"}` |
| `idle` | UI settles after layout/scroll | `{"idle": true}` |
| `scroll` | Scroll state changes | `{"state": "idle"}` |
| `keyboard` | Keyboard shows/hides | `{"visible": true}` |

**Response:** `200 OK`, `Content-Type: text/event-stream`
```
event: activity
data: {"name": "SiteDetailActivity", "state": "resumed"}

event: idle
data: {"idle": true}
```

### POST /animations

Enable or disable animations on the device. Disabling reduces flakiness during TC execution.

**Request body:**
```json
{"enabled": false}
```

**Response:** `200 OK`
```json
{"animations": false}
```

### GET /debug-log

Returns the decision log from the last `/semantic` walk — why each view was included/excluded, what properties were extracted, any extraction failures.

**Response:** `200 OK`, `Content-Type: text/plain`

Per-view decision log. Format is implementation-defined — used for debugging, not automation.

## YAML Schema (required fields)

Every `/semantic` response MUST include these top-level fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `screen` | string | yes | Current activity/screen name |
| `device` | string | yes | Device model identifier |
| `platform` | string | yes | `android` or `ios` |
| `timestamp` | ISO 8601 | yes | Capture time |
| `source` | string | yes | `instrumented` (agent) or `accessibility` (fallback) |
| `viewport` | object | yes | Screen dimensions |
| `viewport.width` | float | yes | Viewport width in dp/pt |
| `viewport.height` | float | yes | Viewport height in dp/pt |
| `viewport.density` | float | yes | Pixel density multiplier |
| `elements` | array | yes | Flat list of UI elements |

## Element Schema

Each element in the `elements` array. Required fields marked with *.

### Identity

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | * | Stable identifier (resource name, accessibility ID, or generated) |
| `platform_id` | string | | Platform-specific view ID |
| `type` | string | * | Element type: `text`, `button`, `image`, `container`, `input`, `toggle`, `list`, `tab`, `icon` |
| `content` | string | | Text content or accessibility label |

### Geometry

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `bounds` | object | * | Bounding box in dp/pt coordinates |
| `bounds.x` | float | * | Left edge |
| `bounds.y` | float | * | Top edge |
| `bounds.w` | float | * | Width |
| `bounds.h` | float | * | Height |
| `z_index` | int | | Stacking order (higher = on top) |

### State

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `clickable` | bool | | Responds to tap |
| `enabled` | bool | | Interactive (not greyed out) |
| `accessible` | bool | | Exposed to accessibility services |
| `a11y_label` | string | | Accessibility label (may differ from content) |
| `visible` | bool | | Within viewport bounds |

### Typography

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `font.family` | string | | Font family name (e.g. "poppins") |
| `font.weight` | string | | Weight: `regular`, `medium`, `semibold`, `bold` |
| `font.size` | float | | Size in sp/pt |
| `line_count` | int | | Number of rendered text lines |
| `truncated` | bool | | Text was truncated (ellipsis) |

### Appearance

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `foreground` | string | | Text/tint color as hex `#RRGGBB` |
| `background` | string | | Background color as hex `#RRGGBB` |
| `corner_radius` | float | | Corner radius in dp/pt |
| `elevation` | float | | Shadow elevation in dp/pt |

### Border

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `border.width` | float | | Border width in dp/pt |
| `border.color` | string | | Border color as hex |

### Gradient

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `gradient.type` | string | | `linear`, `radial`, `sweep` |
| `gradient.colors` | array | | Hex color stops |
| `gradient.orientation` | string | | `top_bottom`, `left_right`, `tl_br`, etc. |

### Spacing

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `margin.top` | float | | Top margin in dp/pt |
| `margin.bottom` | float | | Bottom margin |
| `margin.start` | float | | Start (left in LTR) margin |
| `margin.end` | float | | End (right in LTR) margin |

### Image

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `image.resource` | string | | Resource/asset name |
| `image.type` | string | | `vector` or `raster` |
| `image_path` | string | | Exported image file path |

## Walker Requirements

The view tree walker MUST:

1. **Walk all visible views** — include containers, not just leaf views
2. **Use dp/pt coordinates** — never raw pixels. Divide by density
3. **Exclude system chrome** — status bar, navigation bar, keyboard
4. **Exclude views outside viewport** — offscreen views from scroll containers
5. **Order by z_index** — higher z_index = rendered on top
6. **Extract real font data** — not defaults. Use reflection (Android), accessibility tokens (iOS), or framework internals
7. **Unwrap compound drawables** — RippleDrawable → GradientDrawable → solid color (Android)
8. **Report container bounds** — containers are elements too, not just wrappers
9. **Debounce walks** — don't walk during animations or layout passes. Check idle state first

## Integration Contract

The agent integrates into a host app via two interfaces:

### AgentAuth

```
isAuthenticated() → bool
login(email, password) → void
logout() → void
resetState() → void          # clear tokens, caches, keychain
```

### AgentNavigator

```
createDetailView(id: int) → View/Intent/ViewController
createProfileView(id: int) → View/Intent/ViewController
```

The host app provides concrete implementations. The agent engine has zero imports from the host app — all app-specific logic flows through these interfaces.

## Auto-Start

The agent MUST start automatically when the app launches in debug mode. No manual activation.

- **Android:** `ContentProvider` registered in debug manifest. `onCreate` starts the HTTP server.
- **iOS:** `+load` or `@objc static func` called at launch. Starts HTTP server.
- **Other platforms:** equivalent auto-start mechanism. The operator must not need to manually enable the agent.

## Port Discovery

Default ports (9876 Android, 9877 iOS) are overridable via environment variable. The runner reads the same variable. Port forwarding via `adb forward` (Android) or direct connection (iOS simulator) is the runner's responsibility.

## Compatibility

This spec is implemented by:
- `semantic-agent-android` (Kotlin, Android 7+)
- `semantic-agent-ios` (Swift, iOS 16+)

A conforming agent for any platform (Flutter, React Native, Compose Multiplatform, web) can be built by implementing the HTTP endpoints and YAML schema above. The runners (`ddb`, `idb`) are platform-specific, but a generic runner (`rdb`?) could target this spec directly.
