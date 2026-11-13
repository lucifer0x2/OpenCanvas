#!/bin/bash
# Deploys one git commit: build or reuse its release, back up, switch, restart, verify;
# switches back automatically on any failure. Run by the user, never by an agent.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
# shellcheck source=scripts/native/deploy-common.sh
source "$HERE/deploy-common.sh"

usage() {
  cat <<'EOF'
Usage: deploy.sh <git-ref> [--dry-run] [--yes] [--keep N]

Builds runtime/releases/<sha> from the exact commit with build-release.sh (or reuses an
existing verified release), takes a verified database backup, switches runtime/current,
restarts only the server through process-compose, waits for readiness and runs
verify-runtime.mjs. Any failure switches back to the previous release, restarts and
verifies it, records the failure in runtime/deploy-history.log and exits non-zero.
Deploying the release that is already current does nothing.

  --dry-run  Print the plan and change nothing.
  --yes      Do not ask for confirmation on the live root.
  --keep N   After success, delete releases beyond the N most recently used (N >= 2);
             the current and previous releases are always kept.

Root: OPENCANVAS_ROOT, default ~/Library/Application Support/OpenCanvas.
EOF
}

ref='' DRY_RUN=false ASSUME_YES=false keep=''
while (($#)); do
  case $1 in
    -h | --help) usage; exit 0 ;;
    --dry-run) DRY_RUN=true ;;
    --yes) ASSUME_YES=true ;;
    --keep) [[ $# -ge 2 ]] || die "--keep needs a number"; keep=$2; shift ;;
    --keep=*) keep=${1#--keep=} ;;
    -*) die "Unknown option: $1 (see --help)" ;;
    *) [[ -z "$ref" ]] || die "Only one git ref is accepted"; ref=$1 ;;
  esac
  shift
done
[[ -n "$ref" ]] || { usage >&2; exit 2; }
[[ -z "$keep" || ( "$keep" =~ ^[0-9]+$ && "$keep" -ge 2 ) ]] || die "--keep needs a number of at least 2"
root_init
[[ -L "$RUNTIME/current" && -f "$ROOT/process-compose.yaml" ]] ||
  die "$ROOT is not running under process-compose yet; run migrate-to-process-compose.sh first"
sha=$(git -C "$NATIVE_REPO" rev-parse --verify --quiet "$ref^{commit}") || die "Unknown git ref: $ref"

# Releases ordered by last use (history, newest first), then by age; prints those to delete.
prune_candidates() {
  local current previous original name index=0
  current=$(current_release)
  previous=$(previous_release)
  # The release that migrate-to-process-compose.sh --revert would restore.
  original=$(sed -n 2p "$RUNTIME/process-compose-migration" 2>/dev/null || true)
  {
    [[ ! -f "$HISTORY" ]] || awk -F '\t' '{ line[NR] = $4 "\n" $3 } END { for (i = NR; i > 0; i--) print line[i] }' "$HISTORY"
    ls -t "$RELEASES"
  } | awk '!seen[$0]++' | while IFS= read -r name; do
    [[ -n "$name" && "$name" != .* && -d "$RELEASES/$name" ]] || continue
    index=$((index + 1))
    if ((index > keep)) && [[ "$name" != "$current" && "$name" != "$previous" && "$name" != "$original" ]]; then
      printf '%s\n' "$name"
    fi
  done
}

current=$(current_release)
say "Root: $ROOT"
note "current release: $current"
note "requested:       $sha ($ref)"
if [[ "$current" == "$sha" ]]; then
  say "Release $sha is already current; nothing to do"
  exit 0
fi
source_step="build runtime/releases/$sha with build-release.sh"
[[ ! -d "$RELEASES/$sha" ]] || source_step="reuse runtime/releases/$sha after checking its SHA256SUMS"
if [[ "$DRY_RUN" == true ]]; then
  say "Dry run: nothing will be changed. Plan:"
  note "1. $source_step"
  note "2. back up the database with backup-tools/backup.sh (restore-verified)"
  note "3. switch runtime/current: $current -> $sha"
  note "4. restart only the server through process-compose and wait for readiness"
  note "5. run verify-runtime.mjs; on failure switch back to $current, restart and verify"
  if [[ -n "$keep" ]]; then
    note "6. prune with --keep $keep (after the switch the current set changes; candidates now):"
    prune_candidates | sed 's/^/       /'
  fi
  exit 0
fi

confirm_live "Deploy $sha to the live OpenCanvas runtime at $ROOT?"
lock_acquire
trap lock_release EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP
current=$(current_release)
[[ "$current" != "$sha" ]] || { say "Release $sha became current meanwhile; nothing to do"; exit 0; }

if [[ -d "$RELEASES/$sha" ]]; then
  say "Checking existing release $sha"
  release_valid "$sha" && [[ "$(head -n 1 "$RELEASES/$sha/REVISION")" == "$sha" ]] ||
    die "runtime/releases/$sha fails its checks (REVISION, binaries or SHA256SUMS); move it aside and retry"
else
  say "Building $sha (log: $ROOT/logs/build-$sha.log)"
  "${OPENCANVAS_BUILD_RELEASE:-$HERE/build-release.sh}" "$sha" "$RELEASES/$sha" 2>&1 | tee "$ROOT/logs/build-$sha.log"
  release_valid "$sha" || die "The new release failed its checks"
fi

say "Backing up the database before switching"
"$ROOT/backup-tools/backup.sh" "$NATIVE_CONF"
"$HERE/switch-release.sh" deploy "$sha"

if [[ -n "$keep" ]]; then
  say "Pruning releases beyond the $keep most recently used"
  prune_candidates | while IFS= read -r name; do
    [[ "$name" != "$(current_release)" ]] || continue
    note "removing runtime/releases/$name"
    rm -rf "${RELEASES:?}/$name"
  done
fi
say "Deployed $sha"
