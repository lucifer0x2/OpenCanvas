#!/usr/bin/env bash
# Opt-in public router only. This script never configures Tailscale Serve/Funnel.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
kube=(kubectl --context orbstack --namespace opencanvas-local)

case "${1:-help}" in
  apply)
    # Require an explicit origin every apply; no tracked machine-specific DNS.
    origin=$(node --input-type=module - "${OPENCANVAS_PUBLIC_ORIGIN:-}" <<'JS'
const raw = process.argv[2];
const url = new URL(raw);
if (!/^https:\/\/[^/?#]+\/?$/.test(raw) || url.protocol !== 'https:' || url.username || url.password || url.search ||
    url.hash || !url.hostname || url.pathname !== '/' || /[\s$]/.test(raw))
  throw new Error('OPENCANVAS_PUBLIC_ORIGIN must be an HTTPS origin without credentials/path/query');
console.log(url.origin);
JS
)
    # Start with local.sh apply; never initialize/rotate credentials here.
    "${kube[@]}" get deployment/opencanvas statefulset/postgres >/dev/null
    "${kube[@]}" get secret/postgres-credentials >/dev/null
    config() {
      "${kube[@]}" create configmap opencanvas-public-viewer \
        --from-literal="origin=$origin" --dry-run=client -o yaml
    }
    config | "${kube[@]}" apply --dry-run=server -f -
    "${kube[@]}" apply --dry-run=server -k "$root/deploy/kubernetes/public-viewer"
    config | "${kube[@]}" apply -f -
    "${kube[@]}" apply -k "$root/deploy/kubernetes/public-viewer"
    # The ConfigMap's env value is read at startup, including repeated applies.
    "${kube[@]}" rollout restart deployment/opencanvas
    bash "$root/deploy/kubernetes/local.sh" wait
    ;;
  access)
    # Both forwards share one Pod and must be restarted after its replacement.
    exec "${kube[@]}" port-forward --address 127.0.0.1 service/opencanvas \
      3857:3857 3858:3858
    ;;
  status|wait|restart-app|restart-postgres)
    exec bash "$root/deploy/kubernetes/local.sh" "$1"
    ;;
  *)
    echo 'Usage: OPENCANVAS_PUBLIC_ORIGIN=https://HOST bash deploy/kubernetes/public-viewer.sh apply'
    echo '       bash deploy/kubernetes/public-viewer.sh {access|status|wait|restart-app|restart-postgres}'
    [ "${1:-help}" = help ] || exit 2
    ;;
esac
