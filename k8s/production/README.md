# Production Kubernetes Layout

This directory holds production-grade Kubernetes examples grouped by operating scenario, not by any real tenant or project.

## Scenarios

- `managed-cluster/`
  - opinionated default
  - you own the Kubernetes cluster
  - one managing office governs centrally declared office deployments
  - all managed offices are expected to track the same desired Zebflow image version
- `federated-join/`
  - looser interoperability mode
  - another operator deploys their own office and chooses to join your management
  - deployment lifecycle stays office-owned, not cluster-owned by the manager

## Managed Cluster Topologies

Inside `managed-cluster/`, use the folder name to understand the topology:

- `single/`
  - one standalone office
  - simplest production baseline
- `multi-office/`
  - extends `single/`
  - converts the main office into the manager
  - adds one centrally managed joined office

These manifests are intentionally generic:

- no private tenant names
- no real domains
- no real tokens or passwords

Replace the example secrets before applying anything to a real cluster.
