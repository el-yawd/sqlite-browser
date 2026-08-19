# Specifications

Specifications define observable behavior and constraints. Code, tests, and pull requests should cite stable requirement IDs from these documents.

| Specification | Status | Purpose |
| --- | --- | --- |
| [Product](product.md) | Active | User value, scope, and priorities |
| [Architecture](architecture.md) | Active | Component boundaries and invariants |
| [Quality](quality.md) | Active | Correctness, performance, and release gates |
| [UI interaction fixes](ui-interaction-fixes/requirements.md) | Active | Existing interaction requirements and backlog |
| [Scrolling notes](../notes/scrolling.md) | Historical | Record of the current scrolling implementation |

New work starts from [`_template.md`](_template.md). A specification is only `Done` when its acceptance criteria have evidence. Existing checked tasks in the UI interaction plan are historical assertions until corresponding tests or manual evidence are added.

Validation records live under `docs/validation/`. The current UI baseline is documented in the [GPUI-native exploration from 2026-08-19](../validation/2026-08-19-gpui-native-ui-exploration.md).
