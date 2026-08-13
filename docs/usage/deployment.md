# Deployment

The same Zebflow executable can run in three roles.

- **Standalone** runs the controller and office in one process.
- **Controller** handles platform control and coordination.
- **Office** runs projects and project owned services.

Start with standalone unless you need projects to run on separate workers.

## Durable Data

Set `ZEBFLOW_PLATFORM_DATA_DIR` or mount `/var/lib/zebflow/data` in the container.
Without durable storage, projects disappear with the container or temporary
machine.

## Network

Use a reverse proxy for TLS and public routing. Send project execution routes to
the office that owns the project. Proxy WebSocket routes with upgrade support.
Long jobs should return a job identity and continue outside a short public HTTP
request.

## Health

The main application uses port `10610` by default. A separate health listener
can use `ZEBFLOW_HEALTH_PORT`, commonly `10611`. Use the separate listener for
liveness so a busy application route does not hide whether the process is
alive. Use readiness to decide whether the office should receive traffic.

## Before Production

- use a strong admin password
- keep the data root on durable storage
- back up projects and databases
- set CPU and memory requests based on measured work
- limit upload size at every proxy and at Zebflow
- use explicit outbound network rules
- set log retention
- test restore before relying on backup
- test runtime updates with compatibility preflight
