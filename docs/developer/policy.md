# Policy

Authority:

- `src/platform/policy/mod.rs`
- `src/platform/policy/package.rs`
- `src/platform/policy/report.rs`

Policy is the shared review and warning layer for project material before it is accepted.

Current focus:

- Hub/Add+ package review
- external URLs
- credential requirements
- file writes
- database effects
- schedules
- public endpoints
- risky initialization

Policy should stay centralized so different sources can use the same review shape:

- Hub
- built-in catalog
- Git
- local folders
- local zip archives
