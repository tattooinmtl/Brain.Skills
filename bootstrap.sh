#!/usr/bin/env bash
# bootstrap.sh — clone .skills to ~/.skills (or C:/.skills on Windows via git-bash)
# and run the installer. Safe to re-run: if the repo already exists as a git
# checkout, it just runs `git pull --ff-only` and re-invokes install.js.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/tattooinmtl/Brain.Skills/main/bootstrap.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/tattooinmtl/Brain.Skills/main/bootstrap.sh | bash -s -- --auto
set -euo pipefail

REPO_URL="https://github.com/tattooinmtl/Brain.Skills.git"
# Default install location: /c/.skills on Windows (git-bash), ~/.skills elsewhere.
if [[ "${OS:-}" == "Windows_NT" ]] || [[ -d "/c" ]]; then
  DEST="/c/.skills"
else
  DEST="$HOME/.skills"
fi

command -v git  >/dev/null || { echo "git required" >&2; exit 1; }
command -v node >/dev/null || { echo "node required — https://nodejs.org" >&2; exit 1; }

if [[ -d "$DEST/.git" ]]; then
  echo "Existing checkout at $DEST — pulling latest…"
  git -C "$DEST" fetch --quiet origin
  git -C "$DEST" pull --ff-only
elif [[ -e "$DEST" ]]; then
  echo "ERROR: $DEST exists but is not a git checkout." >&2
  echo "  Move or delete it, or set DEST=... and re-run." >&2
  exit 1
else
  echo "Cloning $REPO_URL to $DEST…"
  git clone --depth 20 "$REPO_URL" "$DEST"
fi

echo "Running installer…"
node "$DEST/install.js" "$@"
