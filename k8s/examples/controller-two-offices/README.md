# Controller + Two Offices (Manual Test)

This example deploys a minimal Zebflow federation test into the `zebflow-test`
namespace:

- `zebflow-controller`
- `zebflow-office-a`
- `zebflow-office-b`
- `debian-test`

The goal is to verify:

- controller boots and is reachable
- offices register themselves to the controller
- controller and offices can see each other over cluster DNS
- direct office runtime URLs exist even before public ingress is configured

This example intentionally stays manual:

- no ingress
- no mTLS yet
- no persistent volumes
- no autosync

It uses the published Docker Hub image:

- `insanalamin/zebflow:0.2.0.202604100244`

## Apply

```bash
KUBECONFIG=~/Dev/talos-hadafid-config/kubeconfig \
kubectl apply -f k8s/examples/controller-two-offices/
```

## Wait For Pods

```bash
KUBECONFIG=~/Dev/talos-hadafid-config/kubeconfig \
kubectl rollout status deployment/zebflow-controller -n zebflow-test

KUBECONFIG=~/Dev/talos-hadafid-config/kubeconfig \
kubectl rollout status deployment/zebflow-office-a -n zebflow-test

KUBECONFIG=~/Dev/talos-hadafid-config/kubeconfig \
kubectl rollout status deployment/zebflow-office-b -n zebflow-test
```

## Port Forward The Controller

```bash
KUBECONFIG=~/Dev/talos-hadafid-config/kubeconfig \
kubectl port-forward -n zebflow-test svc/zebflow-controller 10610:10610
```

Then open:

- `http://127.0.0.1:10610/login`

Default login:

- user: `superadmin`
- password: `admin123`

## Internal Office URLs

These are the cluster-internal URLs the offices advertise to the controller:

- controller:
  - `http://zebflow-controller.zebflow-test.svc.cluster.local:10610`
- office A:
  - `http://zebflow-office-a.zebflow-test.svc.cluster.local:10610`
- office B:
  - `http://zebflow-office-b.zebflow-test.svc.cluster.local:10610`

## Debian Test Pod

Open a shell:

```bash
KUBECONFIG=~/Dev/talos-hadafid-config/kubeconfig \
kubectl exec -it -n zebflow-test pod/debian-test -- bash
```

Install curl inside the pod:

```bash
apt-get update && apt-get install -y curl jq
```

Then verify internal reachability:

```bash
curl -i http://zebflow-controller:10610/login
curl -i http://zebflow-office-a:10610/login
curl -i http://zebflow-office-b:10610/login
```

The office role does not serve the normal controller-style admin workflow, so
`/login` is mainly a simple HTTP reachability check there. The real office
validation is registration in the controller inventory.

## What To Check In UI

After logging into the controller:

1. Open `/home`
2. Create a project and choose `Office A` or `Office B`
3. Open that project's infrastructure page
4. Confirm the office target and office address are shown

## Clean Up

```bash
KUBECONFIG=~/Dev/talos-hadafid-config/kubeconfig \
kubectl delete namespace zebflow-test
```
