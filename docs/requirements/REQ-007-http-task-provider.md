# REQ-007: HTTP REST task provider

- **Status:** Documented, not implemented
- **Added:** 2026-11-05
- **Owner:** maintainer

## Rationale

An external service will expose the active task over a REST API. A consumer provider lets the watcher use that service without changing the poll loop. The service is not yet built, so this page records the intended shape, and the contract details stay TBD until the API exists.

## Design

- New crate `crates/aw-taskwarrior-provider-http` implementing `TaskProvider` (depends on REQ-006). Feature `provider-http` in the watcher.
- `NAME = "http"`.
- Blocking HTTP client, matching the rest of the daemon. Client crate choice is recorded in [../dependencies.md](../dependencies.md) when made.
- Settings under `[providers.http]`:

```toml
provider = "http"

[providers.http]
base_url = "https://example.invalid"   # TBD: API host
timeout_seconds = 5                    # TBD default
api_key = "..."                        # TBD: auth scheme (header name, token source)
```

- Request and response shapes are TBD until the API contract exists. Mapping rules:
  - A successful response with no active task maps to `ActiveTask::default_task()`.
  - Transport errors map to `ProviderError::Unavailable`; HTTP timeouts to `Timeout`; unexpected bodies to `InvalidResponse`.
- Secrets (`api_key`) must not be logged.

## Acceptance criteria

- [ ] With `provider = "http"` and the feature enabled, the watcher reads the active task from the API.
- [ ] Unreachable API: the last known task is reused and the error is logged.
- [ ] Missing or invalid `[providers.http]` settings fail at startup with `InvalidConfig`.
- [ ] `api_key` never appears in logs.

## Code references

- Implemented in: (none yet; blocked on the external API contract)
