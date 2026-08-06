# Cloud Service Native Strategies

This document defines how Zebflow should become easy to install and operate on
AWS, GCP, Azure, and self-hosted cloud-like environments.

This is not a node-compatibility document.

Node support for cloud databases, credentials, queues, and APIs belongs in node
and credential design. This document focuses on Zebflow itself as an
installable full-stack orchestration platform.

## 1. Goal

Zebflow should feel native to the major cloud platforms without becoming locked
to any one of them.

The product should be installable by:

- AWS users
- GCP users
- Azure users
- Kubernetes users
- single-server users
- Raspberry Pi / local-lab users

The same Zebflow core must run across all of them.

## 2. First Principle

Zebflow should expose one stable deployment contract and let each environment
provide implementations around it.

Core deployment contract:

```text
container image
data root
public URL
store
secret provider
ingress
backup/restore
health/upgrade
```

AWS, GCP, Azure, and local installs should differ in how these are implemented,
not in what Zebflow is.

## 3. Primary Artifact: OCI Image

The primary distribution artifact should be an official container image.

Example:

```text
ghcr.io/zebflow/zebflow:<version>
```

The image should run with minimal required configuration:

```text
ZEBFLOW_DATA_ROOT=/var/lib/zebflow/data
ZEBFLOW_PUBLIC_URL=https://example.com
ZEBFLOW_PLATFORM_DEFAULT_PASSWORD=...
```

Everything else should be optional or profile-driven.

## 4. Kubernetes First For Cloud

Kubernetes is the practical common denominator for cloud-native installation:

- AWS EKS
- GCP GKE
- Azure AKS
- self-hosted K3s
- Talos
- bare-metal clusters

Zebflow should ship an official Helm chart.

Chart structure:

```text
charts/zebflow/
  Chart.yaml
  values.yaml
  templates/
    statefulset.yaml
    service.yaml
    ingress.yaml
    pvc.yaml
    secret.yaml
```

Default Kubernetes mode:

```text
single StatefulSet
one PVC
local default Store
platform-managed secrets
```

Advanced Kubernetes modes can enable:

- external object Store
- external secret provider
- managed database
- ingress controller-specific settings
- cloud load balancer annotations

## 5. Install Profiles

Users should not need to assemble the deployment model from scratch.

Zebflow should provide named profiles.

### `standalone`

Single process / single machine.

```text
one binary or one container
local data root
local default Store
local platform metadata
```

Primary target:

- local development
- Raspberry Pi
- small office server
- first install

### `kubernetes-basic`

Cloud-ready but minimal.

```text
one StatefulSet
one PVC
local default Store on mounted volume
Kubernetes Secret
Ingress
```

Primary target:

- EKS/GKE/AKS quick install
- K3s
- internal company cluster

### `kubernetes-cloud-store`

Kubernetes runtime with external artifact storage.

```text
StatefulSet
PVC for data/runtime state
external object Store for files/artifacts
Kubernetes Secret or cloud secret provider
Ingress
```

Primary target:

- production cloud install
- artifact-heavy workflows
- generated media/static/data files

### `cloud-managed`

Managed cloud services around Zebflow.

```text
Kubernetes or managed container runtime
managed object Store
managed secret provider
managed PostgreSQL optional
managed ingress/load balancer
backup integration
```

Primary target:

- enterprise cloud install
- hub listing
- managed production deployment

## 6. Terraform Modules

Each major cloud should have a Terraform module that provisions cloud resources
and installs the Helm chart.

### AWS

Possible resources:

- EKS or ECS
- S3 bucket
- RDS optional
- Secrets Manager optional
- ALB / ingress
- IAM roles
- Helm release

### GCP

Possible resources:

- GKE or Cloud Run
- Cloud Storage bucket
- Cloud SQL optional
- Secret Manager optional
- HTTPS load balancer
- IAM service account
- Helm release

### Azure

Possible resources:

- AKS or Container Apps
- Azure Blob Storage
- Azure Database optional
- Key Vault optional
- Application Gateway / ingress
- managed identity
- Helm release

## 7. Backup, Restore, Upgrade

Cloud-native adoption requires operational confidence.

Zebflow should provide first-class commands or admin flows for:

```text
zebflow health
zebflow backup create
zebflow backup restore
zebflow upgrade check
```

Backup scope must be explicit:

```text
platform/
users/*/repo/
users/*/data/
users/*/files/ or external Store references
services/
libraries/
```

The backup model must respect the formal storage split:

```text
repo/   source
data/   runtime state
files/  Store-backed artifacts
```

## 8. Hub Distribution

After container, Helm, and Terraform contracts are stable, Zebflow can be listed
on cloud hubs.

Targets:

- AWS Hub
- Google Cloud Hub
- Azure Hub

Hub packages should install the same core artifact with provider-specific
defaults.

They should not create cloud-specific Zebflow variants.

## 9. Cloud-Native But Not Cloud-Locked

The product rule:

```text
Zebflow core stays provider-neutral.
Cloud adapters provide implementations.
Install profiles provide defaults.
```

This keeps Zebflow equally viable for:

- local standalone installs
- Kubernetes installs
- AWS-native deployments
- GCP-native deployments
- Azure-native deployments
- self-hosted object storage environments

