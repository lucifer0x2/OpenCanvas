#!/usr/bin/env bash
# All mutations are scoped to the disposable local namespace. Never use set -x:
# the credentials generated below must not appear in terminal output.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
context=orbstack
namespace=opencanvas-local
image=opencanvas:kubernetes-local
kube=(kubectl --context "$context" --namespace "$namespace")

credentials() {
  kubectl --context "$context" apply -f "$root/deploy/kubernetes/base/namespace.yaml"
  if "${kube[@]}" get secret postgres-credentials >/dev/null 2>&1; then
    echo 'Keeping existing PostgreSQL credentials.'
    return
  fi
  if [ -n "$("${kube[@]}" get pvc -o name)" ]; then
    echo 'PVC exists without credentials; restore its Secret instead of rotating the password.' >&2
    exit 1
  fi
  # Hex is safe inside both this YAML string and a PostgreSQL URI.
  local password
  password=$(openssl rand -hex 24)
  printf 'apiVersion: v1\nkind: Secret\nmetadata:\n  name: postgres-credentials\ntype: Opaque\nstringData:\n  POSTGRES_PASSWORD: "%s"\n  DATABASE_URL: "postgresql://opencanvas:%s@postgres:5432/opencanvas"\n' \
    "$password" "$password" | "${kube[@]}" create -f -
  unset password
}

rollout() {
  "${kube[@]}" rollout status statefulset/postgres --timeout=180s
  "${kube[@]}" rollout status deployment/opencanvas --timeout=240s
}

case "${1:-help}" in
  build)
    # OrbStack shares the Docker image store with its Kubernetes node.
    docker --context orbstack build -t "$image" "$root"
    ;;
  apply)
    credentials
    # Validate admission against the real cluster before workload mutations.
    "${kube[@]}" apply --dry-run=server -k "$root/deploy/kubernetes"
    "${kube[@]}" apply -k "$root/deploy/kubernetes"
    # Reused local tags and ConfigMaps do not change a Deployment's Pod spec.
    # Replace the sole Pod so build + apply always runs the newly built image.
    "${kube[@]}" rollout restart deployment/opencanvas
    rollout
    ;;
  wait) rollout ;;
  access)
    # Host and application ports must stay equal for Origin and generated URLs.
    exec "${kube[@]}" port-forward --address 127.0.0.1 service/opencanvas 3857:3857
    ;;
  status)
    "${kube[@]}" get pods,services,pvc
    ;;
  restart-app)
    "${kube[@]}" rollout restart deployment/opencanvas
    "${kube[@]}" rollout status deployment/opencanvas --timeout=240s
    ;;
  restart-postgres)
    "${kube[@]}" rollout restart statefulset/postgres
    "${kube[@]}" rollout status statefulset/postgres --timeout=180s
    "${kube[@]}" wait --for=condition=Ready pod -l app=opencanvas --timeout=240s
    ;;
  stop)
    # Deliberately preserve Namespace, Secret and PVC. Apply resumes them.
    "${kube[@]}" scale deployment/opencanvas statefulset/postgres --replicas=0
    ;;
  *)
    echo 'Usage: bash deploy/kubernetes/local.sh {build|apply|wait|access|status|restart-app|restart-postgres|stop}'
    [ "${1:-help}" = help ] || exit 2
    ;;
esac
