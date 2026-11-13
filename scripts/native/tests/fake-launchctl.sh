#!/bin/bash
# Test double for launchctl (bootstrap, bootout, print) so migration tests never use launchd.
# bootstrap runs the plist's ProgramArguments in the background in its own process group
# with launchd's default signal dispositions; bootout sends SIGTERM like launchd, waits up to
# ExitTimeOut, then kills the job's process group. State lives in FAKE_LAUNCHD_STATE.
set -euo pipefail
state=${FAKE_LAUNCHD_STATE:?FAKE_LAUNCHD_STATE is required}
mkdir -p "$state"
command=${1:?usage: fake-launchctl.sh bootstrap DOMAIN PLIST | bootout DOMAIN/LABEL | print DOMAIN/LABEL}
printf '%s\n' "$*" >> "$state/calls.log"

running() { [[ -f "$state/$1.pid" ]] && kill -0 "$(cat "$state/$1.pid")" 2>/dev/null; }

case $command in
  bootstrap)
    plist=${3:?missing plist}
    label=$(plutil -extract Label raw -o - "$plist")
    if running "$label"; then echo "Bootstrap failed: 5: Input/output error (already loaded)" >&2; exit 5; fi
    arguments=()
    index=0
    while value=$(plutil -extract "ProgramArguments.$index" raw -o - "$plist" 2>/dev/null); do
      arguments+=("$value")
      index=$((index + 1))
    done
    out=$(plutil -extract StandardOutPath raw -o - "$plist")
    err=$(plutil -extract StandardErrorPath raw -o - "$plist")
    timeout=$(plutil -extract ExitTimeOut raw -o - "$plist" 2>/dev/null || printf 20)
    set -m
    "${arguments[@]}" >> "$out" 2>> "$err" < /dev/null &
    printf '%s\n' "$!" > "$state/$label.pid"
    printf '%s\n' "$timeout" > "$state/$label.timeout"
    ;;
  bootout)
    label=${2##*/}
    running "$label" || { echo "Boot-out failed: 3: No such process" >&2; exit 3; }
    pid=$(cat "$state/$label.pid")
    kill -TERM "$pid" 2>/dev/null || true
    deadline=$((SECONDS + $(cat "$state/$label.timeout")))
    while kill -0 "$pid" 2>/dev/null && ((SECONDS < deadline)); do sleep 0.2; done
    kill -KILL -- "-$pid" 2>/dev/null || true
    rm -f "$state/$label.pid" "$state/$label.timeout"
    ;;
  print)
    running "${2##*/}" || exit 113
    printf '%s = { state = running; pid = %s }\n' "$2" "$(cat "$state/${2##*/}.pid")"
    ;;
  *) echo "fake-launchctl: unsupported command $command" >&2; exit 64 ;;
esac
