# Preflight Pipeline Spec

## Overview

The preflight pipeline converts a ticket into executable test cases:

```
ticket (Linear/Jira) 
  → preflight skill (scan sources, compile requirements)
  → preflight-output.yaml (formalized, schema-validated)
  → TC generator (structured YAML → TC YAML)
  → runner (ddb/idb executes TCs on device)
  → results + baseline capture
```

Each stage has a clear contract. No stage parses the previous stage's output with regex.

## Stage 1: Preflight Skill

**Input:** Ticket ID or feature keyword.

**Output:** `tickets/{id}/preflight-output.yaml` conforming to `preflight-output-schema.yaml`.

**The skill is project-specific.** Each project has its own preflight SKILL.md with:
- API base URLs
- Figma file IDs
- Codebase locations (iOS, Android paths)
- Ticket tracker config (Linear project, Jira board)
- Legacy app references

**The output schema is universal.** Every project's preflight produces the same YAML structure. The TC generator doesn't know or care which project generated it.

### What the skill does (per existing SKILL.md phases):

1. **Source scan** — Linear ticket, API spec, Figma, both codebases, legacy apps
2. **Compile** — user-story.md (human-readable) + triage.md
3. **Formalize** — preflight-output.yaml (machine-readable, NEW)
4. **Test data** — probe API for fixture data, identify missing fixtures
5. **QA baseline** — device screenshots of existing implementation

### Phase 3b: Formalize (the new phase)

After compiling user-story.md, extract structured data:

```python
# Conceptual — this runs inside the Claude skill, not as a script

for each FR section in user-story.md:
    fr = {
        id: extract "FR{N}" from heading,
        name: extract heading text after colon,
        type: "interactive" if any state has Requires column, else "static",
        navigation: {
            target: infer from FR context (site_detail, questions_list, etc.),
            via: extract navigation steps from FR description
        },
        state_table: []
    }
    
    for each row in FR's state table:
        state = {
            id: row[0],          # S1, S2, etc.
            state: row[1],       # human-readable name
            condition: row[2],   # what makes this state active
            renders: row[3],     # what UI shows
            requires: row[4] if exists,  # Requires column
            fixture: infer from condition (site-related → test_site, user-related → test_user),
            assert_type: infer from renders (presence → element_exists, absence → element_not_exists),
            assert_target: extract quoted strings from renders column
        }
        fr.state_table.append(state)
    
    # Platform forks
    if FR has platform-specific notes:
        state.platform_fork = {android: ..., ios: ...}
```

### Validation before delivery

The skill validates its own output:

1. Every FR has ≥1 state
2. Every state has renders + assert_target
3. Requires references are valid (exist within the same FR or as FR{N}-S{N})
4. Fixture keys are consistent across FRs (same key = same entity)
5. Navigation targets are from the known set (extensible per project)
6. Coverage summary matches actual FR/state counts

If validation fails → skill reports which FRs/states are incomplete and asks operator to resolve before delivering.

### API base URLs in output

```yaml
api:
  base_url: "https://apiv3.naturkartan.se"
  endpoints:
    - method: GET
      path: "/v3.1/sites/{siteId}/relationships/questions"
      purpose: "Fetch questions for site detail Q&A section"
      auth_required: false
      used_for: runtime
```

The agent's NetworkIdleResource uses these to know which traffic matters. The runner passes them to the agent at TC start: `POST /configure {api_base_urls: ["apiv3.naturkartan.se"]}`. Traffic to other hosts is ignored.

## Stage 2: TC Generator

**Input:** `preflight-output.yaml`

**Output:** TC YAML files in `features/{feature}/tests/`

**Currently:** `generate-tc.py` (Python, parses markdown with regex). Brittle, NK-specific.

**Target:** A Claude skill or a tctl subcommand that reads structured YAML.

### Generator logic (from refactor plan):

```
for each FR in preflight-output.yaml:
    for each state in FR.state_table:
        tc = new TC()
        tc.id = "{FR.id}-{state.id}"
        tc.name = "{FR.name} — {state.state}"
        
        # Navigation
        tc.steps += navigation_steps(FR.navigation)
        
        # Precondition steps from Requires
        if state.requires:
            for req_id in state.requires:
                req_state = lookup(req_id)
                tc.steps += interaction_steps(req_state)
        
        # Interaction (for the current state's condition)
        tc.steps += interaction_steps(state.condition)
        
        # Assertion
        tc.steps += assert_step(state.assert_type, state.assert_target)
        
        # Platform fork
        if state.platform_fork:
            tc.platform_fork = state.platform_fork
        
        # Fixture
        tc.fixture = state.fixture
        
        # Apply gotcha rules (R1-R17 from tc-authoring-guide.md):
        # - wait_idle after navigation
        # - scroll_to before off-screen assert
        # - keyboard dismiss after type
        # - etc.
        tc = apply_authoring_rules(tc)
        
        write(tc, "features/{feature}/tests/{tc.id}.yaml")
```

### Authoring rules (mechanical, from tc-authoring-guide.md):

| Rule | When | What |
|------|------|------|
| R1 | After navigate_to_site/user | Insert wait_idle |
| R2 | Before assert on off-screen element | Insert scroll_to |
| R3 | After type | Insert keyboard dismiss |
| R4 | Interactive flow state | Prepend required states' steps |
| R5 | Platform fork | Wrap in platform block |
| R6 | Fixture ref in target | Use {{fixtures.key.field}} |
| R15 | Requires column | Prepend interaction steps from required states |
| R16 | Element detection | Use idle barrier (wait_for_network) |
| R17 | Hash check | TC requires DDB_EXPECTED_HASH |

These are mechanical — the generator applies them without judgment. The rules come from session learnings (each one traces to a specific failure).

## Stage 3: Runner

**Input:** TC YAML files + fixtures.yaml + navigation.yaml

**Output:** Result YAML + baseline captures

Already built (Phases 0-5). The runner doesn't change for the preflight pipeline — it consumes TC YAML regardless of how they were generated.

## Stage 4: Results + Baseline

**Input:** TC results from runner

**Output:** 
- Result YAML in `features/{feature}/tests/results/`
- Baseline YAML in `features/{feature}/baseline/` (on green runs with --capture-baseline)
- Matrix via `vdb matrix`

## Per-Project Configuration

Each project that uses the pipeline needs:

```yaml
# project.yaml (read by tctl)
project: naturkartan
platforms:
  android:
    package: se.naturkartan.android
    agent_port: 19876
    source: /Users/Shared/projects/Outdoors/nk-android-2026
  ios:
    package: se.outdoormap.naturkartan
    agent_port: 9877
    source: /Users/Shared/projects/Outdoors/nk-ios-2026

api:
  base_url: "https://apiv3.naturkartan.se"
  search_url: "https://ts.naturkartan.se"

ticket_tracker:
  type: linear
  project: OUT

credentials:
  email: ${TEST_EMAIL}
  password: ${TEST_PASSWORD}

devices:
  - name: a54
    platform: android
  - name: dev-ios-one
    platform: ios

catalogue: catalogue/features/
fixtures: catalogue/fixtures.yaml
```

The preflight skill reads project.yaml for API URLs, codebase paths, Figma refs. The TC generator reads it for fixture paths. tctl reads it for device config. One config file, three consumers.

## What's Left to Build

| Item | Effort | Depends on |
|------|--------|------------|
| Add Phase 3b to preflight SKILL.md | 1 hr | Schema (done) |
| Validation step in preflight | 1 hr | Phase 3b |
| TC generator as tctl subcommand (replace generate-tc.py) | 3 hr | Schema (done), authoring rules (done) |
| project.yaml for NK | 30 min | tctl scaffold (done) |
| tctl run reads project.yaml | 2 hr | project.yaml |
| tctl doctor reads project.yaml | 1 hr | project.yaml |
| POST /configure on agent (api_base_urls for network idle filtering) | 1 hr | NetworkIdleResource (done) |
