# SQLite Browser

A native [GPUI](https://www.gpui.rs/) application for inspecting the physical page layout of SQLite database files.

The project currently visualizes database and B-tree page headers, supports page inspection, and refreshes when the open file changes. WAL interpretation, record browsing, and database editing are not yet supported.

## Development

Read [`AGENTS.md`](AGENTS.md) before contributing. Product and engineering requirements live in [`docs/specs/`](docs/specs/README.md).

```sh
script/bootstrap
script/check
cargo run -- path/to/database.db
```

Use `script/quick` while iterating. Profiling output must be created through `script/profile` or written under `target/profiles/`; generated databases and raw performance data are intentionally ignored.
