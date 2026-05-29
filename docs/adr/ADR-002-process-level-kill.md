# ADR-002: Process-Level Kill for WDA Subprocesses

**Status:** Accepted  
**Date:** 2026-05-29  
**Context:** idb test runner (Test.swift)

## Decision

Wrap long-running WDA/curl subprocesses in a process-level timeout with kill.

## Context

The idb runner shells out to `curl` for all WDA and semantic agent communication. When the device disconnects, USB times out, or WDA hangs, the curl process blocks indefinitely. The runner has no way to interrupt it because `shell()` calls `Process.waitUntilExit()` synchronously.

This manifests as:
- `scroll_to` looping 10 times with each curl hanging → 210s+ before timeout
- `findElementViaWDA` scroll+recheck loop hanging on device disconnect
- `enforceLoggedOut` WDA UI flow hanging mid-sequence

The Android runner (ddb) solved this with `spawn()` + timer thread + `kill -9` at 30s. The idb runner needs the same pattern.

## Implementation

**Phase 3 (extract):** Extract WDA communication into a `WDATestClient` that wraps all curl calls with `Process` + `DispatchQueue.asyncAfter` kill.

```swift
func wdaCurl(_ args: String, timeout: TimeInterval = 30) -> (code: Int32, out: String) {
    let proc = Process()
    // ... setup
    proc.launch()
    DispatchQueue.global().asyncAfter(deadline: .now() + timeout) {
        if proc.isRunning { proc.terminate() }
    }
    proc.waitUntilExit()
    // ... return result
}
```

## Consequences

- scroll_to failures terminate in ~30s, not 210s
- Device disconnects don't hang the entire suite
- All WDA timeouts are configurable and consistent
