# REQ-012: HTTP provider without credentials (task-rest local mode)

- **Status:** Documented, not implemented
- **Added:** 2026-11-07
- **Owner:** maintainer

## Rationale

task-rest now has an opt-in local mode (`python -m task_rest.serve --no-auth`, task-rest `b7386eb`). In that mode there is no `/auth` route, token headers are ignored, and the server reads the running user's own TaskWarrior data. A single-user machine does not need a password for the daemon to read its own tasks.

## Design

### Settings

`username` and `password` become optional, and must be set together:

```toml
[aw-taskwarrior.providers.http]
base_url = "http://127.0.0.1:5000"   # required
# no username/password: task-rest local mode (--no-auth)
```

- Both set: login with `GET /auth` (Basic), then the token, as in REQ-007.
- Neither set: no `/auth` request and no `x-access-tokens` header. The list request is sent directly.
- Only one set: `InvalidConfig` naming both keys ("username and password must be set together").

### Requests

- With credentials: `GET /auth`, then `GET /?status=pending&start.any=` with the token.
- Without credentials: `GET /?status=pending&start.any=` with no token. No `/auth`.

### Errors

| Condition | Result |
|-----------|--------|
| No credentials, server answers `401` | `Unavailable("server requires authentication: set username and password")` |
| Credentials set, `GET /auth` answers `404` | `Unavailable("HTTP 404 from /auth: wrong base_url, or the server runs without authentication (remove username and password)")` |
| Credentials set, wrong password | `Unavailable("authentication rejected: check username and password")` (unchanged) |

Secrets (password, token) never appear in logs or errors, as in REQ-007.

### Security

- task-rest's local mode is loopback-only by default (Host header check; a non-loopback bind needs a typed confirmation). The provider does not change that and sends no credentials it does not have.
- Without credentials, the provider reads whatever the server exposes. Use it only with a local-mode server on a trusted machine. Do not point `base_url` at a server that another person runs.

## Acceptance criteria

- [ ] With no username and password, the provider reads the active task from a `--no-auth` task-rest, and never calls `/auth`.
- [ ] With only one of username and password, startup fails with `InvalidConfig` naming both keys.
- [ ] A `401` without credentials reports the "server requires authentication" message.
- [ ] A `404` from `/auth` with credentials reports the hint message.
- [ ] The existing wrong-password message and the secrets rule are unchanged.

## Code references

- Provider: (none yet; to be implemented in `crates/aw-taskwarrior-provider-http`)
- Server: task-rest `task_rest/serve.py`, `task_rest/__init__.py` (`create_app(auth_enabled=False)`), commit `b7386eb`.
