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

Known-issue probes:

```sh
cargo test --locked interaction_tests -- --ignored --nocapture
```

The known-issue command is expected to fail until the corresponding behavior is implemented.

## Working behavior

| Scenario | Evidence | Result |
| --- | --- | --- |
| Click a rendered page cell | Synthetic click on `page-2`; grid and sidebar both report page 2 | Pass |
| Dismiss an error status | Synthetic click on the rendered error banner; current status becomes empty | Pass |
| Dismiss a file-open error | Synthetic click on `file-dialog-dismiss`; dialog returns to idle/open state | Pass |

## Confirmed interaction issues

### UI-001: Large page grids cannot be scrolled

**Priority:** Critical usefulness issue

With 200 rendered pages, a wheel event over `page-grid-container` does not change the last page's position. The grid uses `overflow_hidden`, so pages below the viewport are clipped and unreachable.

Related requirements: `PROD-002`, UI interaction requirement 4.4.

Regression probe: `scrolling_moves_later_pages_into_view`.

### UI-002: Progress messages are accidentally dismissible

**Priority:** High correctness issue

Clicking the body of a progress message removes it even though the message is marked non-dismissible. The rendered Cancel control has no action handler; its click bubbles to a blanket handler around the whole status element and only dismisses the message.

Related requirements: `PROD-006`, UI interaction requirements 4.5 and 5.5.

Regression probe: `progress_status_cannot_be_dismissed_by_clicking_its_body`.

### UI-003: Success and informational messages never auto-dismiss

**Priority:** Medium usability issue

Advancing GPUI's deterministic clock beyond four seconds leaves a success message visible. `StatusManager` stores the configured duration but does not schedule dismissal, so transient messages also block queued messages until the user clicks them.

Related requirement: UI interaction requirement 5.2.

Regression probe: `success_status_auto_dismisses_after_three_seconds`.

### UI-004: Keyboard page navigation is absent

**Priority:** Medium accessibility issue

Sending a Right Arrow key with a populated grid leaves selection empty. The browser owns an unused focus handle, the grid does not track focus, and no key handlers are registered.

Related requirement: UI interaction requirements 6.1–6.5.

Regression probe: `right_arrow_selects_the_first_page`.

## Additional high-risk findings from the exercised paths

These findings come from tracing the handlers reached by the native tests and require focused lifecycle tests:

1. File parsing runs synchronously inside `cx.spawn`, which runs on GPUI's foreground thread. Large files can freeze rendering and input.
2. The parse cancellation flag is not passed to the parser used by `FileManager::open_file_with_progress`, and it is not cleared after completion. Cancellation and operation-state reporting are therefore unreliable.
3. The native `rfd` picker is called synchronously inside a foreground GPUI task and bypasses GPUI's simulated path-selection API, preventing deterministic open-dialog testing.
4. Status action controls are rendered without handlers. `handle_status_action` is currently unreachable from the rendered status UI.
5. The header has an Open File control but no rendered refresh control or keyboard bindings despite registered action types.
6. File-watcher retry scheduling emits a failure event instead of actually starting another watcher attempt; the browser then marks watching as failed.

## Recommended order

1. Make parsing and reloads background tasks with real cancellation and stale-result suppression.
2. Replace the clipped page grid with a scrollable or virtualized implementation.
3. Wire status actions precisely and implement deterministic auto-dismiss tasks.
4. Add focus tracking, arrow navigation, and Open/Refresh key bindings.
5. Abstract file selection behind an injectable interface so the full open/cancel/error flow can be tested with GPUI's test platform.
