# Preflight Skill Update Spec

The preflight skill (`.claude/skills/preflight/SKILL.md`) already lists `preflight-output.yaml` in its output directory but has no generation logic for it.

## What Needs to Change

Add a **Phase 3b: Generate Structured Output** after Phase 3 (Compile User Story) and before Phase 4 (Test Data).

### Phase 3b: Generate preflight-output.yaml

After the user-story.md is compiled (Phase 3), convert the FR state tables into the formalized schema.

**Input:** user-story.md (compiled in Phase 3) — contains FR state tables in markdown.

**Output:** `preflight-output.yaml` conforming to `tctl/docs/preflight-output-schema.yaml`.

**Process:**

1. Read the compiled user-story.md
2. Extract each FR section (FR1, FR2, etc.)
3. For each FR, extract:
   - id, name, type (static/interactive)
   - navigation path (from the FR description)
   - States table: id, name, condition, requires, renders, fixture ref
4. Map renders to element entries: `{element: "text", type: button|text|link|input|image}`
5. For interactive FRs: populate the `requires` field from the Requires column
6. Write `preflight-output.yaml`

**Validation:** Before writing, validate against the schema:
- Every FR has at least one state
- Every state has at least one renders entry
- Requires references valid state IDs within the same FR
- Fixture refs match keys in test-data.md

**Example transformation:**

```markdown
## FR6: Q&A Section on Site Detail

| # | State | Condition | What renders |
|---|-------|-----------|-------------|
| S1 | has-questions | site has questions | "Questions & Answers" header, question cards, "See all questions" link |
| S2 | no-questions | site has 0 questions | "Questions & Answers" header, "No questions yet" text |
```

→

```yaml
- id: FR6
  name: "Q&A Section on Site Detail"
  type: static
  navigation: "site detail → scroll to Q&A"
  states:
    - id: S1
      name: has-questions
      condition: "site has questions"
      requires: []
      renders:
        - element: "Questions & Answers"
          type: text
        - element: "question cards"
          type: list
        - element: "See all questions"
          type: link
      fixture: test_site
    - id: S2
      name: no-questions
      condition: "site has 0 questions"
      requires: []
      renders:
        - element: "Questions & Answers"
          type: text
        - element: "No questions yet"
          type: text
      fixture: empty_site
```

## TC Generator Consumption

The TC generator (currently `generate-tc.py`, target: tctl skill) reads `preflight-output.yaml` instead of parsing markdown. No regex. No table detection. Structured YAML → structured YAML.

Pipeline: `preflight skill → preflight-output.yaml → TC generator → TC YAMLs → runner`

## What Does NOT Change

- user-story.md (human-readable) still produced — unchanged
- All other phases (1-5) unchanged
- test-cases.md still produced (human-readable TC matrix)
- The preflight-output.yaml is an ADDITIONAL output, not a replacement
