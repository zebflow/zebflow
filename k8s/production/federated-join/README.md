# Federated Join

This scenario is for an independently deployed office that chooses to join an existing managing office.

Use this when:

- you do not own the manager's Kubernetes cluster
- the office operator controls their own deployment lifecycle
- version compatibility matters, but centralized auto-update does not

This example assumes:

- one office deployment
- one PVC for local office data
- one Service exposing the office inside its own cluster
- one join secret containing the local fallback password and the join token

## Files

- `namespace.yaml`
- `zebflow-office-secrets.example.yaml`
- `office.yaml`

## Secret

Copy the example and replace all placeholders:

```bash
cp k8s/production/federated-join/zebflow-office-secrets.example.yaml /tmp/zebflow-office-secrets.yaml
```

## Apply

```bash
kubectl apply -f k8s/production/federated-join/namespace.yaml
kubectl apply -f /tmp/zebflow-office-secrets.yaml
kubectl apply -f k8s/production/federated-join/office.yaml
```

## Required Edits Before Apply

Replace these values in `office.yaml`:

- office id
- office label
- manager URL
- public advertise URL for this office

## Notes

- This example does not install auto-update. The office operator owns lifecycle and upgrade policy.
- The managing office should treat this office by contract compatibility, not by assuming shared cluster ownership.
- The office keeps its own PVC and runtime state locally even while management is delegated.
