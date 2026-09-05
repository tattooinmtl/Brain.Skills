#!/usr/bin/env bash
# install.sh — Unix native launcher for Brain.Skills
set -e
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

if [[ -f "$DIR/bin/skills" ]]; then
  exec "$DIR/bin/skills" install "$@"
elif [[ -f "$DIR/bin/skills.exe" ]]; then
  exec "$DIR/bin/skills.exe" install "$@"
elif command -v go >/dev/null 2>&1; then
  echo "Compiling native Go launcher..."
  go build -o "$DIR/bin/skills" "$DIR/agent-skills-installer/cmd/agent-installer"
  exec "$DIR/bin/skills" install "$@"
else
  echo "skills binary not found in bin/ and Go is not installed." >&2
  exit 1
fi
