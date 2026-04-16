#!/usr/bin/env bash
set -euo pipefail

cargo fetch --locked

if ! command -v rg >/dev/null 2>&1; then
  echo "[init] warning: rg is not installed; docs/scripts/check_links.sh will fail until the mission addresses this." >&2
fi

if ! docker ps >/dev/null 2>&1; then
  echo "[init] warning: Docker is unavailable; Docker-backed sandbox validation is currently blocked." >&2
fi
