# Managed Cluster

This is the opinionated production scenario for operators who control the Kubernetes cluster.

The managed-cluster policy is:

- one managing office governs centrally declared office deployments
- managed offices use the same desired Zebflow image tag
- rollout skew is tolerated briefly during update
- office lifecycle is cluster-owned, not office-owned
- auto-update is optional and should be treated as an explicit operator choice, not the default

## Source Of Truth

The preferred source of truth is the cluster folder itself:

- `management.yaml`
- `office-main.yaml`
- one file per joined office such as `office-a.yaml`
- `auto-update.yaml`

These files are meant to be managed by:

- `zebflow k8s cluster init`
- `zebflow k8s cluster add-office`
- `zebflow k8s cluster set-controller`
- `zebflow k8s cluster set-image`
- `zebflow k8s cluster enable-auto-update`
- `zebflow k8s cluster disable-auto-update`
- `zebflow k8s cluster describe`
- `zebflow k8s cluster validate`

Zebflow only rewrites the sections marked as Zebflow-owned inside those files. The rest of each YAML
file is left alone so you can still add cluster-specific config in place.

```bash
zebflow k8s cluster init ./greenpeace
zebflow k8s cluster add-office ./greenpeace office-a
zebflow k8s cluster set-image ./greenpeace insanalamin/zebflow:0.2.0.202604120235
zebflow k8s cluster enable-auto-update ./greenpeace
zebflow k8s cluster describe ./greenpeace
zebflow k8s cluster validate ./greenpeace
```

## Topologies

- [`single/`](./single/README.md)
  - one standalone office
  - simplest production base
- [`multi-office/`](./multi-office/README.md)
  - extends `single/`
  - converts the main office into the manager
  - adds one or more centrally managed offices

Use this scenario when you want Zebflow to be the cluster standard, not just one app among unrelated tenant deployments.
