# Documentation status

Drift register. Every requirement in [requirements/](requirements/README.md) appears here with its docs status and code status.

Rules:
- `Docs ahead` (docs done, code not done) is valid, and only for work in progress.
- `Code ahead` is a defect. Update the docs first.
- `In sync`: both match.

## Requirements

| ID | Title | Docs status | Code status | Drift |
|----|-------|-------------|-------------|-------|
| REQ-001 | `--testing` flag | Documented | Not implemented | Docs ahead |
| REQ-002 | CLI overrides | Documented | Not implemented | Docs ahead |
| REQ-003 | Log levels | Documented | Not implemented | Docs ahead |
| REQ-004 | Health metrics | Documented | Not implemented | Docs ahead |
| REQ-005 | Event categories | Documented | Not implemented | Docs ahead |
| REQ-006 | Plugin-style providers (workspace crates) | Implemented | Implemented | In sync |
| REQ-007 | HTTP REST task provider | Implemented | Implemented | In sync |
| REQ-008 | In-process library task provider | Documented | Not implemented | Docs ahead |
| REQ-009 | ActivityWatch config convention | Implemented | Implemented | In sync |
| REQ-010 | Server-side task filtering in task-rest | Documented | Not implemented | Docs ahead |
| REQ-011 | Cross-compile for Windows 11 from Linux | Implemented | Implemented | In sync |

## Known code issues (not requirements)

| Issue | Where | Status |
|-------|-------|--------|
| `task` runs twice per poll: once to wait, once to collect output | `crates/aw-taskwarrior-provider-cli/src/lib.rs` `CliProvider::query` / `run_task_export_with_timeout` | Open |
| Clippy `field_reassign_with_default` warning in test code | `crates/aw-taskwarrior/src/activitywatch.rs` (test `test_select_pulsetime_custom_offline_tag_name`) | Open |

Code-side fixes for these should be preceded by a doc entry describing the intended behavior.
