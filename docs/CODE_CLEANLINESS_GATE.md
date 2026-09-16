# ClinicDesk Code Cleanliness Gate

This gate is mandatory before large new feature work or the network-mode implementation.

## Goal

Improve maintainability without changing observable product behavior, database semantics, or established safety rules.

## Rules

1. Refactor in small, reviewable commits.
2. Preserve behavior before and after every refactor.
3. Do not remove functionality merely to reduce line count.
4. Do not suppress useful compiler, Clippy, TypeScript, lint, security, or test warnings.
5. Remove dead or duplicate code only after verifying it is unused.
6. Prefer cohesive components/modules with a single clear responsibility over oversized page files.
7. Extract shared UI and domain helpers only when they represent real repeated behavior; avoid abstraction for its own sake.
8. Keep React UI independent of SQLite-specific implementation details so future network mode can use the same frontend boundary.
9. Keep database migrations deterministic and data-preserving.
10. A failed required check blocks the next cleanup slice.

## Cleanup order

1. Establish a source-size and responsibility inventory.
2. Format/readability cleanup of compressed frontend sources with no behavior change.
3. Split oversized patient UI responsibilities into focused components/helpers.
4. Split oversized appointment UI responsibilities into focused components/helpers.
5. Consolidate genuinely shared date/time, status-label, modal, and action helpers where appropriate.
6. Audit imports, unreachable/dead code, duplicate logic, stale TODOs, and unused dependencies.
7. Audit Rust modules for oversized responsibilities, duplication, avoidable allocations/clones, error consistency, and unsafe state coupling.
8. Audit SQL migrations and schema helpers for duplication, obsolete compatibility code, and deterministic upgrade behavior.
9. Run the complete project quality/security gate.

## Required checks after each behavior-affecting slice

- Frontend build/type checking.
- Existing frontend lint/test checks when present.
- `cargo fmt --check`.
- `cargo clippy --all-targets -- -D warnings` (or the repository's stricter equivalent).
- Rust tests.
- Migration/upgrade tests.
- Existing GitHub Actions build matrix.
- CodeQL and dependency/security checks applicable to the change.

## Exit criteria

The cleanup gate is complete only when:

- no known dead/duplicate code remains without an explicit reason;
- major UI files have focused responsibilities and are readable/reviewable;
- no useful warning has been disabled to obtain a green build;
- existing functionality and data semantics remain intact;
- required CI/security checks are green;
- a final whole-project review finds no cleanup blocker before network-mode work begins.
