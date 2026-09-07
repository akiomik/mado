#!/usr/bin/env bash
#
# Print the version in `Cargo.toml`. Everything else a release carries a version
# in has to agree with this one, so everything that needs it reads it here.
#
# Usage: version.sh

set -euo pipefail

cd "$(dirname "$0")/../.."

# Read the `[package]` table, not the first `version =` in the file: the
# workspace tables above it can grow one of their own, and a dependency pinned
# in table form would otherwise be read as mado's version.
#
# Take what is between the quotes, or the bare value if there are none, rather
# than whatever `cut` makes of a line with no delimiter in it.
version=$(sed -n '/^\[package\]$/,/^\[/{
  s/^version = "\{0,1\}\([^"]*\)"\{0,1\}$/\1/p
}' Cargo.toml | head -1)
if [ -z "$version" ]; then
  echo "$0: Cargo.toml has no version" >&2
  exit 1
fi

printf '%s\n' "$version"
