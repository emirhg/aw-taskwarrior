# REQ-002: CLI arguments for host, port and API key override

- **Status:** Documented, not implemented
- **Added:** 2026-11-05
- **Owner:** maintainer

## Rationale

Running against a non-default ActivityWatch server currently needs a config file. Command-line overrides make one-off runs simpler.

## Design

- Parse `--host`, `--port` and `--api-key` in `main.rs`. Values override the loaded config.
- Precedence: CLI argument, then config file, then built-in default.
- Affects: `main.rs`, `config.rs`. See [../modules.md](../modules.md).
- Needs an argument parser crate or hand-written parsing; record the choice in [../dependencies.md](../dependencies.md) when made.

## Acceptance criteria

- [ ] `--host` and `--port` override `aw_server_host` and `aw_server_port`.
- [ ] `--api-key` overrides `api_key` and is not printed in logs.
- [ ] With no flags, behavior is unchanged.

## Code references

- Implemented in: (none yet)
