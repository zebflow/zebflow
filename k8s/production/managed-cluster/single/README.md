# Managed Cluster: Single Office Base

This is the simplest production topology:

- one namespace: `zebflow`
- one PVC: `zebflow-data`
- one Zebflow deployment running in `standalone` mode
- one internal `ClusterIP` service
- optional support services:
  - `browserless`
  - `searxng`

The settings mirror the Talos production shape:

- `strategy: Recreate`
- `storageClassName: local-path`
- control-plane toleration
- PVC-backed data at `/var/lib/zebflow/data`
- auto-updater CronJob

## Files

- `namespace.yaml`
- `zebflow-secrets.example.yaml`
- `deployment.yaml`
- `auto-update.yaml`
- `browserless.yaml`
- `searxng.yaml`

`auto-update.yaml` is optional. Leave it unapplied unless you explicitly want automatic image tracking.

## Secret

Copy the example and replace the placeholder password:

```bash
cp k8s/production/managed-cluster/single/zebflow-secrets.example.yaml /tmp/zebflow-secrets.yaml
```

## Apply

```bash
kubectl apply -f k8s/production/managed-cluster/single/namespace.yaml
kubectl apply -f /tmp/zebflow-secrets.yaml
kubectl apply -f k8s/production/managed-cluster/single/deployment.yaml
kubectl apply -f k8s/production/managed-cluster/single/browserless.yaml
kubectl apply -f k8s/production/managed-cluster/single/searxng.yaml
```

Optional:

```bash
kubectl apply -f k8s/production/managed-cluster/single/auto-update.yaml
```

## Verify

```bash
kubectl get all -n zebflow
kubectl get pvc -n zebflow
kubectl logs -n zebflow deploy/zebflow --tail=50
kubectl port-forward -n zebflow svc/zebflow 10610:10610
```

Then open `http://127.0.0.1:10610/login`.

## Next Step

If you later want one manager plus multiple centrally managed offices in the same cluster, continue with [`../multi-office/README.md`](../multi-office/README.md).
