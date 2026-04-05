#!/usr/bin/env bash
set -euo pipefail

docs_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$docs_root"

files=()
while IFS= read -r file; do
  files+=("$file")
done < <(find . -type f -name '*.md' ! -path './assets/*' ! -path './stylesheets/*' | sort)

{
  echo "# openai-agents-rust docs summary"
  echo
  echo "Compact export of the markdown docs tree."
  echo
  for file in "${files[@]}"; do
    title="$(awk '/^# / {sub(/^# /, ""); print; exit}' "$file")"
    summary="$(awk '
      BEGIN {in_code=0}
      /^```/ {in_code = !in_code; next}
      in_code {next}
      /^# / {next}
      /^$/ {if (seen) exit; next}
      {
        if (!seen) {
          printf "%s", $0
          seen=1
        } else {
          printf " %s", $0
        }
      }
      END {print ""}
    ' "$file")"
    echo "- ${file#./}: ${title:-Untitled}"
    if [[ -n "$summary" ]]; then
      echo "  ${summary}"
    fi
  done
} > llms.txt

{
  echo "# openai-agents-rust docs full export"
  echo
  for file in "${files[@]}"; do
    echo
    echo "## ${file#./}"
    echo
    cat "$file"
    echo
  done
} > llms-full.txt
