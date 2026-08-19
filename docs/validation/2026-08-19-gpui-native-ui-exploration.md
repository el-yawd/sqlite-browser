# GPUI-native UI exploration

**Date:** 2026-08-19
**Platform:** Linux test platform supplied by GPUI
**GPUI revision:** `aabce921e321efa9c9b14d68367d69a0c43c175b`
**Method:** Rendered test windows, stable debug selectors, synthetic mouse and keyboard input, and entity-state assertions

## Commands

Active interaction checks:

```sh
cargo test --locked interaction_tests -- --nocapture
```

Full repository harness:

```sh
./script/check
```

## Working behavior

| Scenario | Evidence | Result |
| --- | --- | --- |
| Click a rendered page cell | Synthetic click on `page-2`; grid and sidebar both report page 2 | Pass |
| Dismiss an error status | Synthetic click on the rendered error banner; current status becomes empty | Pass |
| Dismiss a file-open error | Synthetic click on `file-dialog-dismiss`; dialog returns to idle/open state | Pass |
| Scroll a 200-page database | Synthetic wheel input moves later pages toward the viewport | Pass |
| Click a progress-message body | Non-dismissible progress remains visible | Pass |
| Auto-dismiss success status | Deterministic clock advancement removes the message | Pass |
| Navigate pages with keyboard | Right Arrow selects the first page and synchronizes the sidebar | Pass |
| Cancel native file selection | Injected picker cancellation returns the dialog to idle | Pass |

## Resolved findings

| Finding | Resolution | Regression evidence |
| --- | --- | --- |
| UI-001: Large page grids were clipped | The constrained content pane now uses tracked vertical scrolling | `scrolling_moves_later_pages_into_view` |
| UI-002: Progress messages were accidentally dismissible | Body dismissal is conditional, and action/dismiss controls have precise handlers with propagation stopped | `progress_status_cannot_be_dismissed_by_clicking_its_body` |
| UI-003: Transient messages never auto-dismissed | `StatusManager` schedules ID-scoped timers on GPUI's background executor | `success_status_auto_dismisses_after_three_seconds` |
| UI-004: Keyboard navigation was absent | The browser tracks focus and maps arrow keys to ordered grid navigation | `right_arrow_selects_the_first_page` |
| Parsing and reloads blocked the foreground | Initial parsing and watcher reload parsing now run on the background executor | Full harness and code-path review |
| Cancellation and stale results were unreliable | Parse generations, cancellation flags, time limits, and load generations suppress stale completion and report cancellation | Full harness and code-path review |
| File selection blocked the foreground and was untestable | An injectable `FilePicker` runs on the background executor | `cancelling_the_native_picker_returns_to_idle` |
| Status controls had no handlers | Action and dismiss controls now route to `handle_status_action` independently | Progress interaction checks |
| Refresh UI and shortcuts were absent | The header now renders Refresh, with scoped `Ctrl+O` and `Ctrl+R` bindings | Full harness and code-path review |
| Watcher retries were reported as failures | Retry timers now emit a distinct retry event that re-enters watcher setup; failure is emitted only after exhaustion | Full harness and code-path review |

## Result

`./script/check` passes with 19 tests, including all former known-issue probes. No interaction test remains ignored.
