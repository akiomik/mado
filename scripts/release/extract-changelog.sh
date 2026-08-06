#!/usr/bin/env bash
#
# Print the CHANGELOG.md section for one version, for use as a GitHub Release
# body. Without this the whole changelog would end up in every release.
#
# Usage: extract-changelog.sh VERSION [CHANGELOG]
#
#   $ scripts/release/extract-changelog.sh 0.3.1

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "usage: $0 VERSION [CHANGELOG]" >&2
  exit 2
fi

version="$1"
changelog="${2:-CHANGELOG.md}"

if [ ! -f "$changelog" ]; then
  echo "$0: no such file: $changelog" >&2
  exit 1
fi

# Compare the heading by prefix rather than by regex, so that a version is
# matched literally: `0.3.2` must not match `## [0.3.20]`, and the `]` in the
# prefix is what rules that out. Leading blank lines are dropped; the trailing
# ones go with the command substitution below.
#
# The link definitions end the last section, which no heading follows.
section=$(
  awk -v version="$version" '
    BEGIN { heading = "## [" version "]" }
    /^## / { inside = (substr($0, 1, length(heading)) == heading); next }
    /^\[[^]]+\]:[ \t]/ { inside = 0; next }
    inside && NF { started = 1 }
    inside && started { print }
  ' "$changelog"
)

if [ -z "$section" ]; then
  echo "$0: no section for version $version in $changelog" >&2
  exit 1
fi

printf '%s\n' "$section"
