# ADR-001: Idle Barrier Over Polling

**Status:** Accepted  
**Date:** 2026-05-29  
**Context:** idb test runner (Test.swift)

## Decision

Replace polling-based idle detection with an event-driven idle barrier pattern (Detox-style).

## Context

The runner currently has two idle-wait functions with different semantics:
- `waitIdle()` (line 1070): polls `/idle` at 0.2s intervals up to `stepTimeout`
- `waitForIdle()` (line 1125): polls `/idle` at 0.5s intervals, max 5 attempts, plus fixed sleeps

Both poll the same endpoint. The duplication causes inconsistent wait behavior: post-action waits (waitForIdle) are capped at ~3s while general waits (waitIdle) can extend to stepTimeout. Neither is optimal.

Industry pattern (Detox, Espresso): the agent maintains an idle resource registry. The runner doesn't poll — it asks the agent to notify when idle. This eliminates:
- Fixed sleep padding (0.3s + 0.5s in waitForIdle)
- Poll interval tuning (0.2s vs 0.5s)
- Redundant HTTP requests

## Implementation

**Phase 2 (consolidation):** Merge waitIdle + waitForIdle into one function with configurable timeout.

**Phase 5 (idle barrier):** Add `POST /query-when-idle` to the semantic agent. The runner sends a query + idle resource list + timeout. The agent waits for idle resources to settle, queries the view tree once, and returns the result. No polling from the runner side.

## Consequences

- Passing TCs complete 3-5x faster (no poll overhead)
- Failing TCs fail at the timeout boundary, not after N×pollInterval
- Runner code simplified: one idle-wait function, eventually replaced by barrier query
