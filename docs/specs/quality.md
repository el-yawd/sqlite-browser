# Quality Specification

**Status:** Active

## Usefulness gates

- **QUAL-001:** Every non-trivial change cites a product or feature requirement and describes its user-visible outcome.
- **QUAL-002:** Scope excludes unrelated refactors unless they are necessary to make the behavior testable or safe.

## Correctness gates

- **QUAL-010:** `script/check` passes from a clean checkout with the locked dependency graph.
- **QUAL-011:** Parser changes test valid input, malformed input, truncation at relevant boundaries, and special SQLite values such as the 65,536-byte page-size encoding.
- **QUAL-012:** No production path intentionally panics on file content, user input, entity lifetime, or I/O failure.
- **QUAL-013:** Fallible operations are propagated or logged with enough context to diagnose the operation and path.
- **QUAL-014:** Tests are deterministic, isolated from the network after dependency fetch, and do not depend on developer-owned databases.
- **QUAL-015:** Manual UI validation records the platform, scenario, expected result, and observed result.

## Performance gates

- **QUAL-020:** Optimization starts with a named workload and a reproducible baseline.
- **QUAL-021:** Compare at least five samples before and after under the same release profile and environment.
- **QUAL-022:** Report median elapsed time and peak memory where relevant; retain commands and summary statistics, not raw profiler output.
- **QUAL-023:** A performance change must preserve all correctness gates.
- **QUAL-024:** Profiler data is written under `target/profiles/` and never committed.

## Repository and review gates

- **QUAL-030:** Generated databases, flamegraphs, traces, editor state, and build output are not tracked.
- **QUAL-031:** Tracked blobs larger than 5 MiB require a documented exception and repository-owner approval.
- **QUAL-032:** Pull requests include requirement IDs, validation evidence, risks, and performance evidence when performance is claimed.

## Release gate

Until automated UI coverage exists, releases require `script/check` plus manual validation of file open/cancel, invalid file handling, page selection, refresh after external modification, deletion of the watched file, and clean application shutdown.

The current harness allows Rust's `dead_code` and `unused_imports` lints because the existing application contains partially wired UI work. All other warnings are denied. Removing these temporary exceptions is tracked as engineering debt; new code should not add to either category.
