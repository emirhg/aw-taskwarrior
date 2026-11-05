# aw-taskwarrior documentation

Reference documentation for the aw-taskwarrior daemon. Pages describe the code as it is on `main`.

| Page | Contents |
|------|----------|
| [architecture.md](architecture.md) | System context, startup, poll loop, shutdown, design decisions |
| [modules.md](modules.md) | Each source module: responsibility, public API, tests, internal dependencies |
| [dependencies.md](dependencies.md) | Crate dependencies and runtime dependencies on external programs and services |
| [requirements/README.md](requirements/README.md) | Requirement index (docs-first: requirements are written here before code) |
| [STATUS.md](STATUS.md) | Drift register: docs status vs. code status for every requirement |

## Status legend

- `Documented, not implemented`: the docs describe it, the code does not have it yet. Allowed.
- `Implemented`: the code and the docs match.
- `Code ahead of docs`: a defect. Fix the docs first.

## Rule

Docs are always at least as far ahead as the code. See the "Documentation-first workflow" section in `CLAUDE.md`.
