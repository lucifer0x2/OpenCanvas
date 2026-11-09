#!/bin/bash
# Install backup files/plist. Loading the LaunchAgent is an explicit next action.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=scripts/native/backup-common.sh
source "$HERE/backup-common.sh"
[[ $# -ge 1 && $# -le 2 ]] || backup_die "Usage: install-backup.sh CONFIG [LAUNCHAGENTS_DIR]"
backup_load_config "$1"
launchagents=${2:-$HOME/Library/LaunchAgents}
[[ "$BACKUP_CONFIG" == /* && "$launchagents" == /* ]] || backup_die "Config and LaunchAgents paths must be absolute"
for directory in "$OPENCANVAS_ROOT" "$OPENCANVAS_ROOT/backup-tools" "$OPENCANVAS_ROOT/logs" "$launchagents"; do
  [[ ! -L "$directory" ]] || backup_die "Install directories cannot be symlinks"
  mkdir -p "$directory"
done
chmod 700 "$OPENCANVAS_ROOT" "$OPENCANVAS_ROOT/backup-tools" "$OPENCANVAS_ROOT/logs"
for tool in backup-common.sh backup-fingerprint.sh backup.sh restore-verify.sh; do
  install -m 700 "$HERE/$tool" "$OPENCANVAS_ROOT/backup-tools/$tool"
done

# XML escaping is separate from shell quoting; no shell-expanded plist command.
xml_escape() { sed -e 's/\&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g"; }
root_xml=$(printf '%s' "$OPENCANVAS_ROOT" | xml_escape)
config_xml=$(printf '%s' "$BACKUP_CONFIG" | xml_escape)
plist="$launchagents/org.opencanvas.backup.plist"
temporary=$(mktemp "$launchagents/.opencanvas-backup.XXXXXXXX")
trap 'rm -f "$temporary"' EXIT
# awk replacement strings interpret ampersands; index/substr avoid substitution.
awk -v root="$root_xml" -v config="$config_xml" '
  { line=$0
    while (p=index(line,"@@ROOT@@")) line=substr(line,1,p-1) root substr(line,p+8)
    while (p=index(line,"@@CONFIG@@")) line=substr(line,1,p-1) config substr(line,p+10)
    print line
  }' "$HERE/backup.launchd.plist.in" > "$temporary"
plutil -lint "$temporary" >/dev/null
chmod 600 "$temporary"
mv "$temporary" "$plist"
printf 'Installed backup tools and org.opencanvas.backup.plist; LaunchAgent has not been loaded.\n'
