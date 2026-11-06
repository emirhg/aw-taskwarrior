# REQ-010: Server-side task filtering in task-rest

- **Status:** Documented, not implemented (the request is written up in task-rest as `REQUEST.md`)
- **Added:** 2026-11-06
- **Owner:** maintainer

## Rationale

`GET /` in task-rest returns every task. On the development data that is 581 tasks, about 1.6 MB per request, and the server runs a full `task export` each time. The HTTP provider (REQ-007) polls every 5 seconds to find the active task. Until the server can filter, every poll moves the whole list.

The companion request is in the task-rest repository, in `REQUEST.md`. It asks for TaskWarrior-style attribute filters as query parameters, for example `GET /?start.any=&status=pending`, which maps to `task start.any: status:pending`.

## Design

- The server change belongs to the task-rest repository. This repository only records it and the provider change that depends on it.
- Once the server accepts filters, the HTTP provider adds the filter to the list request. The request then becomes `GET /?start.any=&status=pending`, and the response is only the active tasks.
- The provider keeps the client-side selection rule (REQ-007) as a safety check, so the result is the same with or without the filter.
- No new setting is needed. The provider sends the filter when the server is known to support it. How that is detected (a setting, or a version check on the server) is decided when this requirement is implemented.

## Acceptance criteria

- [ ] `GET /?start.any=&status=pending` returns the same tasks as `task start.any: status:pending` on the same data (task-rest tests).
- [ ] The HTTP provider sends the filter and returns the same `ActiveTask` as the unfiltered path, on the same data.
- [ ] A poll transfers only the active tasks, not the full list.

## Code references

- Server side: `~/workspace/task-rest/REQUEST.md` (not yet implemented)
- Provider side: (none yet; depends on REQ-007)
