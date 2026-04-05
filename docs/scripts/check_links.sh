#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

status=0

while IFS= read -r file; do
  while IFS= read -r target; do
    clean="${target%%#*}"
    if [[ -z "$clean" ]]; then
      continue
    fi
    if [[ "$clean" =~ ^https?:// ]] || [[ "$clean" =~ ^mailto: ]]; then
      continue
    fi
    resolved="$(cd "$(dirname "$file")" && python3 - <<'PY' "$clean"
import os
import sys
target = sys.argv[1]
print(os.path.normpath(os.path.abspath(target)))
PY
)"
    if [[ ! -e "$resolved" ]]; then
      echo "broken link in $file -> $target"
      status=1
    fi
  done < <(rg -o '\[[^]]+\]\(([^)]+)\)' "$file" -r '$1')
done < <(find . \
  \( -path './.git' -o -path './target' \) -prune -o \
  -type f \( -name '*.md' -o -name 'README.md' \) -print | sort)

exit "$status"
