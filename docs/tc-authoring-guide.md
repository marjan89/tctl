# TC Authoring Guide

Rules for writing YAML test cases that pass on first run. Derived from 6 days of iteration across 97+ toolchain fixes.

## Structural Rules

Every TC must have:

```yaml
id: TC-{number}
name: "short description"  # under 60 chars
precondition:
  activity: MainActivity    # always specify
  logged_in: true           # if any step needs auth
steps:
  - ...
```

## The 14 Rules

### R1: wait_idle after navigation
Every navigate_to_site, navigate_to_user, or screen-changing tap MUST be followed by wait_idle. The agent endpoint returns before the UI renders.

```yaml
- action: navigate_to_site
  site_id: 31255
- action: wait_idle
  seconds: 5
```

### R2: scroll_to before below-fold content
Never assert or tap an element that might be below the viewport without scrolling first.

```yaml
- action: scroll_to
  target: {content_fuzzy: "questions & answers"}
- assert: element_exists
  target: {content_fuzzy: "questions & answers"}
```

### R3: every tap needs a target
Never write `action: tap` without a target. The runner rejects it.

### R4: exclude_type: input after search
When tapping a search result, the search field also matches. Use exclude_type.

```yaml
- action: tap
  target: {content_fuzzy: "sandhammaren", exclude_type: input}
```

### R5: platform fork for destructive gestures
Long-press (Android) vs trash button (iOS) for delete. Use platform fork.

```yaml
- platform:
    android:
      - action: long_press
        target: {content_fuzzy: "{{fixtures.test_user.name}}"}
    ios:
      - action: tap
        target: {content_fuzzy: "remove this question"}
```

### R6: navigate_to_site uses agent endpoint
No manual search-and-tap sequences. The runner calls POST /navigate/site/{id}.

```yaml
- action: navigate_to_site
  site_id: 31255
```

### R7: use fixture refs, never literals
All site names, user names, question text, emails, passwords come from fixtures.yaml.

```yaml
# YES
target: {content_fuzzy: "{{fixtures.test_site.name}}"}
# NO
target: {content_fuzzy: "Sandhammaren"}
```

### R8: search disambiguation
content_fuzzy matches are greedy. "questions" matches "Latest Questions and Answers" header AND the "Questions" tab. Use the longest unambiguous substring.

### R9: keyboard dismiss before tap
After a type action, the keyboard may cover the next tap target. Add wait_idle (runner auto-dismisses keyboard before tap).

```yaml
- action: type
  text: "question text"
- action: wait_idle
  seconds: 3
- action: tap
  target: {id: "submitButton"}
```

### R10: back action uses keyevent
`action: back` sends KEYCODE_BACK. If keyboard is up, it dismisses keyboard first. If no keyboard, it navigates back. Don't use tap on "back" text — use the action.

### R11: AlertDialog detection
The runner checks uiautomator (/sdcard/ui.xml) + dumpsys activity top + semantic agent for element_exists assertions. AlertDialogs are visible to uiautomator but NOT to the semantic agent.

### R12: clickable ancestor
Text inside a clickable parent (MaterialButton wrapping TextView) reports clickable: true via ancestor check. The runner uses tap_target (parent center) when present.

### R13: scroll settle
After scroll_to finds an element, the runner waits 500ms for scroll deceleration before the next step. Don't add extra waits.

### R14: Samsung uiautomator quirk
`uiautomator dump /dev/tty` returns EMPTY on Samsung when AlertDialog is showing. The runner uses `/sdcard/ui.xml` which works.

### R15: interactive flow chaining via Requires column
State tables for interactive flows (post, delete, answer) must use a Requires column. States that depend on prior interaction steps list them by number.

```markdown
| # | State | Requires | Condition | What renders |
|---|-------|----------|-----------|-------------|
| 1 | form-open | — | tap "Post a question" | text field visible |
| 2 | form-valid | 1 | type 2+ chars | submit enabled |
| 3 | form-submitted | 1,2 | tap submit | "thank" dialog |
```

The generator reads Requires and prepends the interaction steps from states 1 and 2 before state 3's assertion. Without this, the generator emits navigate+assert without the interaction steps in between.

### R16: SSE-first element detection
The runner now uses SSE /stream for element_exists assertions instead of polling. One quick check, then subscribe to /stream for up to 10s. Failing asserts complete in ~10s instead of ~90s.

### R17: mandatory expected hash
DDB_EXPECTED_HASH or --expected-hash is required. The runner refuses to start without it. Prevents running TCs against a stale APK.

## Step Ordering Template

Every TC follows this pattern:

```yaml
# 1. Navigate to target screen
- action: navigate_to_site
  site_id: {{fixtures.test_site.id}}

# 2. Wait for load
- action: wait_idle
  seconds: 5

# 3. Scroll to target area
- action: scroll_to
  target: {content_fuzzy: "target section"}

# 4. Assert precondition (verify before interact)
- assert: element_exists
  target: {content_fuzzy: "expected element"}

# 5. Interact
- action: tap
  target: {content_fuzzy: "button"}

# 6. Wait for result
- action: wait_idle
  seconds: 5

# 7. Assert result
- assert: element_exists
  target: {content_fuzzy: "success indicator"}
```

## Fixture Reference Patterns

```yaml
{{fixtures.test_site.name}}       # not "Sandhammaren"
{{fixtures.test_site.id}}         # not 31255
{{fixtures.test_user.name}}       # not "sinisa"
{{fixtures.test_user.email}}      # not "sinisa@outdoormap.com"
{{fixtures.test_question.text}}   # not "Is the beach..."
{{fixtures.other_user.name}}      # not "Thomas Ivung"
{{fixtures.oscar.name}}           # not "Oscar Kockum"
{{fixtures.empty_site.name}}      # not "Stora Hjälmmossen"
```

## Runner Action Vocabulary

| Action | Description |
|--------|-------------|
| navigate_to_site | POST /navigate/site/{id} via agent |
| navigate_to_user | POST /navigate/user/{id} via agent |
| tap | input swipe 50ms at target center |
| type | adb input text (auto-clears field first) |
| scroll_to | iterative scroll until target in viewport |
| scroll | scroll N times in direction |
| wait | fixed sleep (seconds) |
| wait_idle | poll /idle until UI settled |
| back | KEYCODE_BACK |
| long_press | input swipe 1500ms at target center |
| capture | screenshot to file |
| api_call | curl to external API |

## Assert Vocabulary

| Assert | Description |
|--------|-------------|
| element_exists | SSE-first: quick check, then /stream 10s wait |
| element_not_exists | inverse of element_exists |
| element_state | check enabled/clickable state by ID |
