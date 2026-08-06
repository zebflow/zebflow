# Distribution

Authoritative locations:

- `npm/`
- `pip/`
- `docker/`
- `charts/`
- `k8s/`
- `libraries/`
- `composites/`
- `src/platform/services/hub.rs`
- `src/platform/help/guide/hub/`

Distribution surfaces:

| Surface | Purpose |
|---|---|
| npm | Simple install for Node.js users. |
| pip | Simple install for Python users. |
| Docker | Server and repeatable deployment. |
| Helm/charts | Kubernetes deployment. |
| libraries | Zeb/RWE frontend library bundles. |
| composites | Built-in composite node packages. |
| Hub | Shareable packages across projects/instances. |

README rule:

- Keep npm and pip prominent because they are easy for many users.
- Docker and source build remain available but should not be the only path.

Hub rule:

- Use `Hub`, not marketplace.
- Use `package` for shareable items.
- Use `node bundle` for installable node packages.
- Add+ is the project action; Hub is one source behind Add+.
