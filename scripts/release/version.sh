#!/usr/bin/env bash
#
# Print the version in `Cargo.toml`. Everything else a release carries a version
# in has to agree with this one, so everything that needs it reads it here.
#
# Usage: version.sh

set -euo pipefail

cd "$(dirname "$0")/../.."

# Take what is between the quotes, or the bare value if there are none, rather
# than whatever `cut` makes of a line with no delimiter in it.
version=$(sed -n '1,/^version = /s/^version = "\{0,1\}\([^"]*\)"\{0,1\}$/\1/p' \
  Cargo.toml | head -1)
if [ -z "$version" ]; then
  echo "$0: Cargo.toml has no version" >&2
  exit 1
fi

printf '%s\n' "$version"
