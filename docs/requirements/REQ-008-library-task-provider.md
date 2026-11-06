# REQ-008: In-process library task provider

- **Status:** Documented, not implemented
- **Added:** 2026-11-05
- **Owner:** maintainer

## Rationale

Some task managers can be read in-process through a library, which avoids spawning a process per poll and avoids the HTTP round trip. A library provider gives the same plug-in point as the CLI and HTTP providers.

## Design

- New crate `crates/aw-taskwarrior-provider-lib` implementing `TaskProvider` (depends on REQ-006). Feature `provider-lib` in the watcher.
- `NAME = "lib"`.
- The library to wrap is TBD. Its choice and license go in [../dependencies.md](../dependencies.md) when made.
- Settings under `[aw-taskwarrior.providers.lib]` (REQ-009): TBD, depending on the library's data location.
- Errors from the library map to `Unavailable` (cannot open the data store) or `InvalidResponse` (data cannot be read).

## Acceptance criteria

- [ ] With `provider = "lib"` and the feature enabled, the watcher reads the active task through the library.
- [ ] The library is not linked unless the feature is enabled.

## Code references

- Implemented in: (none yet; library choice TBD)
