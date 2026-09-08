#!/usr/bin/env bash
#
# Print the version in `Cargo.toml`. Everything else a release carries a version
# in has to agree with this one, so everything that needs it reads it here.
#
# Usage: version.sh

set -euo pipefail

cd "$(dirname "$0")/../.."

# The `[package]` table, not the first `version =` in the file: a dependency
# pinned in table form above it would otherwise be read as mado's version.
version=$(sed -n '/^\[package\]$/,/^\[/{
  s/^version = "\{0,1\}\([^"]*\)"\{0,1\}$/\1/p
}' Cargo.toml | head -1)
if [ -z "$version" ]; then
  echo "$0: Cargo.toml has no version" >&2
  exit 1
fi

printf '%s\n' "$version"
