#!/usr/bin/env bash
# Prepares the release commit: bumps the manifest and closes the changelog's
# Unreleased section under the new version.
#
#   scripts/prepare-release.sh 0.1.0
#
# Commit the result, then tag it. `.github/workflows/release.yml` does the rest.
set -euo pipefail

version="${1:-}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.+][0-9A-Za-z.-]+)?$ ]]; then
  echo "usage: $0 <version>   e.g. $0 0.1.0" >&2
  exit 2
fi

cd "$(dirname "$0")/.."
repo="https://github.com/hupe1980/rutmf"
today=$(date -u +%Y-%m-%d)

if grep -qF "## [$version]" CHANGELOG.md; then
  echo "error: CHANGELOG.md already has a [$version] section" >&2
  exit 1
fi
if ! grep -qF '## [Unreleased]' CHANGELOG.md; then
  echo "error: CHANGELOG.md has no [Unreleased] section to close" >&2
  exit 1
fi
# Releasing an empty section would produce a release with no notes.
if [ -z "$(awk '/^## \[Unreleased\]/{f=1;next} f&&/^## /{exit} f&&NF' CHANGELOG.md)" ]; then
  echo "error: the [Unreleased] section is empty" >&2
  exit 1
fi

# 1. Close Unreleased under the version, and open a fresh one above it.
python3 - "$version" "$today" <<'PY'
import re, sys
version, today = sys.argv[1], sys.argv[2]
text = open("CHANGELOG.md").read()
text = text.replace(
    "## [Unreleased]\n",
    f"## [Unreleased]\n\n## [{version}] - {today}\n",
    1,
)
open("CHANGELOG.md", "w").write(text)
PY

# 2. Point the link references at the new version.
python3 - "$version" "$repo" <<'PY'
import re, sys
version, repo = sys.argv[1], sys.argv[2]
text = open("CHANGELOG.md").read()
text = re.sub(
    r"^\[Unreleased\]: .*$",
    f"[Unreleased]: {repo}/compare/v{version}...HEAD\n"
    f"[{version}]: {repo}/releases/tag/v{version}",
    text,
    count=1,
    flags=re.M,
)
open("CHANGELOG.md", "w").write(text)
PY

# 3. Bump the manifest, and let cargo refresh the lockfile entry.
python3 - "$version" <<'PY'
import re, sys
version = sys.argv[1]
text = open("Cargo.toml").read()
text = re.sub(r'^version = "[^"]+"', f'version = "{version}"', text, count=1, flags=re.M)
open("Cargo.toml", "w").write(text)
PY
cargo metadata --format-version 1 --offline > /dev/null

echo "prepared $version"
git --no-pager diff --stat -- CHANGELOG.md Cargo.toml Cargo.lock
