#!/bin/bash
# Install/load one launchd service. App stays on rehearsal ports until explicit cutover.
set -euo pipefail
umask 077
service=${1:?usage: launch-runtime.sh postgres|server [ROOT]}
root=${2:-$HOME/Library/Application Support/OpenCanvas}
[[ "$service" == postgres || "$service" == server ]]
agents="$HOME/Library/LaunchAgents"
label="org.opencanvas.$service"
config="$root/config/native.conf"
[[ "$service" != server ]] || config="$root/config/server.env"
mkdir -p "$agents"
plist="$agents/$label.plist"
[[ ! -e "$plist" ]] || { echo 'LaunchAgent already exists; refusing overwrite' >&2; exit 1; }
xml() { sed -e 's/\&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'; }
safe_root=$(printf '%s' "$root" | xml)
safe_config=$(printf '%s' "$config" | xml)
cat > "$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>$label</string>
<key>ProgramArguments</key><array><string>/bin/bash</string><string>$safe_root/runtime/scripts/run-$service.sh</string><string>$safe_config</string></array>
<key>RunAtLoad</key><true/><key>KeepAlive</key><true/>
<key>ThrottleInterval</key><integer>5</integer>
<key>ExitTimeOut</key><integer>20</integer>
<key>StandardOutPath</key><string>$safe_root/logs/$service.stdout.log</string>
<key>StandardErrorPath</key><string>$safe_root/logs/$service.stderr.log</string>
</dict></plist>
EOF
chmod 600 "$plist"
plutil -lint "$plist"
launchctl bootstrap "gui/$(id -u)" "$plist"
echo "Loaded $label"
