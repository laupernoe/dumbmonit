#!/bin/sh
# Lists, per language, the documentation pages translated and the ones still
# missing (served in English with a notice). Source of truth: docs/**/*.md
# (English) against docs/**/<name>.<locale>.md.
#
#   scripts/docs-i18n-status.sh            summary + missing pages
#   scripts/docs-i18n-status.sh -q         summary only
set -eu
cd "$(dirname "$0")/.."

LOCALES="fr de es it pt pt-BR ru zh-Hans"
quiet=0
[ "${1:-}" = "-q" ] && quiet=1

# English pages: *.md without a locale suffix.
pages=$(find docs -name '*.md' | while read -r f; do
  base=${f%.md}
  case "${base##*.}" in
    fr|de|es|it|pt|pt-BR|ru|zh-Hans) ;;
    *) echo "${base}" ;;
  esac
done | sort)
total=$(echo "$pages" | wc -l | tr -d ' ')

for l in $LOCALES; do
  done_n=0
  missing=""
  for p in $pages; do
    if [ -f "$p.$l.md" ]; then
      done_n=$((done_n + 1))
    else
      missing="$missing ${p#docs/}.md"
    fi
  done
  printf '%-8s %3d/%d translated\n' "$l" "$done_n" "$total"
  if [ "$quiet" = 0 ]; then
    for m in $missing; do printf '    missing: %s\n' "$m"; done
  fi
done
