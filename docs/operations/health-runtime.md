# Health and Runtime

Authority:

- `src/infra/health.rs`
- `src/bin/zebflow.rs`
- `src/platform/help/platform/operations.md`

Common endpoints:

```text
GET /health
GET /ready
GET /health/live
GET /health/runtime
```

Operational meaning:

- liveness should show whether the process heartbeat is alive
- readiness should show whether the main app can serve traffic
- Kubernetes should not route project traffic to an unready worker

Important failure mode:

A process can be alive while the main runtime is blocked. Dedicated health paths exist so liveness can be separated from readiness.
