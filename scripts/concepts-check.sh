#!/usr/bin/env bash
# Local only: concepts/ and specs/ are gitignored, so the roadmap half does not run in CI.
# 1. Nothing published cites a path into concepts/, specs/, .specify/ or .claude/: its
#    reader cannot open it. The two test loaders that *read* concepts/reference/ by default
#    (overridable with $RUTMF_REFERENCE) are the one exemption, matched exactly.
# 2. Every roadmap item (### `#anchor`) in concepts/ROADMAP.md names one spec folder on a
#    **Specified as:** line, and every folder under specs/ (outside archive/) is named once.
set -euo pipefail
cd "$(dirname "$0")/.."
fail=0

leaks=$(grep -rnE '(^|[^A-Za-z0-9_./-])(concepts/|specs/|\.specify/|\.claude/)' \
    README.md CHANGELOG.md src examples site/content tests --exclude-dir=fixtures 2>/dev/null \
    | grep -vF '.join("concepts/reference")' || true)
[ -z "$leaks" ] || { echo "✗ the published tree cites a path its reader does not have:" >&2; echo "$leaks" | sed 's/^/  /' >&2; fail=1; }

roadmap=concepts/ROADMAP.md
if [ ! -f "$roadmap" ]; then
    [ "$fail" -eq 0 ] || exit 1
    echo "✓ no published path into concepts/, specs/ or .specify/ (no local roadmap to check)"
    exit 0
fi
items=$(grep -cE '^### `#[a-z0-9-]+`' "$roadmap" || true)
named=$(grep -oE '^\*\*Specified as:\*\* `[0-9]{3}-[a-z0-9-]+`' "$roadmap" | sed -E 's/.*`([^`]+)`/\1/' | sort || true)
n=$(printf '%s\n' "$named" | grep -c . || true)
[ "$items" -eq "$n" ] || { echo "✗ $roadmap: $items items but $n **Specified as:** lines" >&2; fail=1; }
dupes=$(printf '%s\n' "$named" | uniq -d)
[ -z "$dupes" ] || { echo "✗ named by more than one item: $dupes" >&2; fail=1; }
for s in $named; do [ -f "specs/$s/spec.md" ] || { echo "✗ $roadmap names $s, but specs/$s/spec.md does not exist" >&2; fail=1; }; done
for d in specs/[0-9][0-9][0-9]-*/; do
    [ -d "$d" ] || continue; s=$(basename "$d")
    printf '%s\n' "$named" | grep -qx "$s" || { echo "✗ specs/$s is named by no roadmap item" >&2; fail=1; }
done
[ "$fail" -eq 0 ] || exit 1
echo "✓ $items roadmap items, each with its specification; no published path into concepts/, specs/ or .specify/"
