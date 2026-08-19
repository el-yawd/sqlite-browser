# Product Specification

**Status:** Active

## Purpose

SQLite Browser is a native desktop tool for understanding the physical layout of SQLite database files while those files are changing. Its primary audience is developers and database practitioners diagnosing storage behavior.

## Product principles

1. Opening a valid database should quickly produce useful page-level information.
2. Inspecting a database must never modify it.
3. Invalid, partial, inaccessible, or changing files must produce clear feedback rather than crashes or stale data.
4. Live refresh should help users without making the interface unreliable.
5. Performance work follows measured user impact and cannot weaken correctness.

## Core requirements

- **PROD-001:** The application opens a user-selected SQLite database in read-only mode and displays its header and physical pages.
- **PROD-002:** The user can select a page and inspect its type, cell count, free space, fragmentation, and relevant pointers.
- **PROD-003:** The application detects external changes to the open database and refreshes the displayed snapshot.
- **PROD-004:** Unsupported, missing, malformed, truncated, or inaccessible files result in an actionable user-visible error.
- **PROD-005:** A parse failure never replaces the last valid displayed snapshot with partial or stale data presented as current.
- **PROD-006:** The UI remains responsive while opening and refreshing databases large enough to require perceptible parsing time.
- **PROD-007:** The application clearly identifies unsupported SQLite features instead of implying complete interpretation.

## Current scope

The current implementation visualizes database-header and B-tree page-header metadata. Editing records, executing SQL, interpreting WAL state, and complete recovery of corrupted databases are out of scope until separately specified.

## Success signals

- Valid databases open without crashes.
- Malformed input tests cover parser boundaries.
- Users can distinguish loading, current, stale, and failed states.
- Representative parsing and rendering workloads have reproducible measurements before performance claims are made.
