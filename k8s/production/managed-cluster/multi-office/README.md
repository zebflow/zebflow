# Managed Cluster: Multi-Office Overlay

This topology extends [single](../single/README.md) into the opinionated managed-cluster layout:

- the existing `deployment/zebflow` becomes the controller
- one joined office is added as `zebflow-office-1`
- the existing PVC `zebflow-data` stays attached to the main deployment
- one centralized updater policy keeps all managed Zebflow deployments on the same desired image tag

This is the managed-cluster bundle for:

- one controller
- one additional office
- shared join contract through `zebflow-cluster-secrets`
- centralized auto-update across all managed offices in the namespace

If you need more than one joined office, duplicate `zebflow-office-1.yaml` with a new office id and PVC name, then add the new deployment name to `TARGET_DEPLOYMENTS` in `zebflow-managed-auto-update.yaml`.

## Files

- `zebflow-cluster-secrets.example.yaml`
- `zebflow-main-service.yaml`
- `zebflow-live-controller.yaml`
- `zebflow-office-1.yaml`
- `zebflow-managed-auto-update.yaml`

`zebflow-managed-auto-update.yaml` is optional. The managed-cluster examples now assume auto-update is off unless you opt in.

## Assumption

Apply this after the `single` bundle already exists. This is a topology upgrade, not a separate namespace.

## Secret

Create the shared join token secret first:

```bash
cp k8s/production/managed-cluster/multi-office/zebflow-cluster-secrets.example.yaml /tmp/zebflow-cluster-secrets.yaml
```

## Apply

```bash
kubectl apply -f /tmp/zebflow-cluster-secrets.yaml
kubectl apply -f k8s/production/managed-cluster/multi-office/zebflow-main-service.yaml
kubectl apply -f k8s/production/managed-cluster/multi-office/zebflow-live-controller.yaml
kubectl apply -f k8s/production/managed-cluster/multi-office/zebflow-office-1.yaml
```

Optional:

```bash
kubectl apply -f k8s/production/managed-cluster/multi-office/zebflow-managed-auto-update.yaml
```

## Verify

- controller internal address:
  - `http://zebflow-main.zebflow.svc.cluster.local:10610`
- joined office internal address:
  - `http://zebflow-office-1.zebflow.svc.cluster.local:10610`

Then log in to the controller and confirm the runtime target list shows:

- `Local office`
- `Office 1`

## Notes

- This causes a brief restart of the main `zebflow` pod because the deployment stays `Recreate`.
- Public ingress does not change automatically in this step.
- Project traffic should only move to the joined office after explicit placement and sync.
- The managed-cluster opinion is: one desired Zebflow image tag for all centrally managed offices, with short rollout skew tolerated during update.
