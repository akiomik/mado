#!/usr/bin/env bash
#
# Check that every file the release tag has to carry the version in agrees with
# `Cargo.toml`. A tag whose `action/entrypoint.sh` or README pins were left
# behind publishes cleanly and hands everyone using the GitHub Action the
# previous release's binary.
#
# Given a tag, check that too, which is all CD can do about a tag pushed at a
# commit no pull request ever checked.
#
# Usage: check-versions.sh [TAG]
#
#   $ scripts/release/check-versions.sh v0.3.1

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
cd "$SCRIPT_DIR/../.."

version=$(bash "$SCRIPT_DIR/version.sh")

mismatch=0

if [ $# -gt 0 ]; then
  tag="$1"
  if [ "${tag#v}" != "$version" ]; then
    echo "$0: tag $tag, Cargo.toml says $version" >&2
    mismatch=1
  fi
fi

entrypoint=$(sed -n 's/^DEFAULT_VERSION="\{0,1\}\([^"]*\)"\{0,1\}$/\1/p' \
  action/entrypoint.sh | head -1)
if [ -z "$entrypoint" ]; then
  echo "$0: no DEFAULT_VERSION this can read in action/entrypoint.sh" >&2
  mismatch=1
elif [ "$entrypoint" != "v$version" ]; then
  echo "$0: action/entrypoint.sh says $entrypoint, Cargo.toml says $version" >&2
  mismatch=1
fi

lock=$(sed -n '/^name = "mado"$/{n;s/^version = "\{0,1\}\([^"]*\)"\{0,1\}$/\1/p;}' \
  Cargo.lock | head -1)
if [ -z "$lock" ]; then
  echo "$0: Cargo.lock has no mado package to check" >&2
  mismatch=1
elif [ "$lock" != "$version" ]; then
  echo "$0: Cargo.lock says $lock, Cargo.toml says $version" >&2
  mismatch=1
fi

# Every pin, not only the ones shaped like a version: `@main` or `@v0.3` has to
# fail here too. Cut each one at the first character a ref cannot contain, so
# the Markdown around it — a link, a backtick, a full stop — is not read as
# part of the version.
pins=$(grep -oE 'akiomik/mado@[^[:space:]]+' README.md |
  cut -d '@' -f 2 | sed 's|[^A-Za-z0-9._/-].*$||; s/\.*$//' | sort -u || true)
if [ -z "$pins" ]; then
  echo "$0: README.md has no akiomik/mado pin to check" >&2
  mismatch=1
fi
for pin in $pins; do
  if [ "$pin" != "v$version" ]; then
    echo "$0: README.md pins $pin, Cargo.toml says $version" >&2
    mismatch=1
  fi
done

exit $mismatch
