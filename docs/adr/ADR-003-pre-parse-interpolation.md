# ADR-003: Pre-Parse Fixture Interpolation

**Status:** Accepted  
**Date:** 2026-05-29  
**Context:** idb test runner (Test.swift)

## Decision

Fixture template interpolation (`{{fixtures.*}}`) runs on the raw YAML string before parsing, not after.

## Context

TC YAML files use `{{fixtures.test_site.id}}` for fixture references. The YAML parser expects `site_id` and `user_id` fields as strings (or integers). If interpolation runs after parsing, the template string `{{fixtures.test_site.id}}` is already parsed as a string — integer fields like `site_id: {{fixtures.test_site.id}}` fail because the YAML parser sees a string where it expects an integer.

The Android runner (ddb) hit this first: `serde_yaml::from_str` rejected `user_id: '{{fixtures.test_user.id}}'` as a non-integer. The fix: raw string replacement before YAML parsing.

The idb runner already does pre-parse interpolation (line 133: `content = interpolateFixtures(content, fixtures: fixtures)` before `parseSpec(content, path: specPath)`). This ADR documents and ratifies the pattern.

## Implementation

Already implemented in idb. The `interpolateFixtures()` function (line 1733) uses regex to replace `{{fixtures.KEY.FIELD}}` patterns in the raw YAML string. The result is then passed to `parseSpec()`.

Key design choices:
- Missing fixture keys are left as-is (no crash, no empty string)
- Regex pattern: `\{\{fixtures\.([a-zA-Z0-9_]+)\.([a-zA-Z0-9_]+)\}\}`
- Replacements applied in reverse order to preserve string positions

## Consequences

- Integer fields (`site_id`, `user_id`) work with fixture references
- Fixture values can contain any valid YAML content
- Missing fixtures produce descriptive error messages at runtime, not parse failures
