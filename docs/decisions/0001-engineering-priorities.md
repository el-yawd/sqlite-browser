# 0001: Prioritize usefulness, correctness, then performance

**Status:** Accepted

## Context

The application handles untrusted binary files in an interactive UI. Fast but misleading or fragile behavior has negative value, while process documents without user outcomes do not improve the product.

## Decision

Engineering trade-offs are ordered: usefulness, correctness, performance. Performance changes require measurement and must preserve specified behavior and correctness gates.

## Consequences

Feature work begins with an observable user outcome. Parser and lifecycle behavior receive deterministic tests. Optimizations may be rejected when they add complexity without a representative measured benefit.
