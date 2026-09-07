prev_version := "0.3.1"
version := "0.3.2"
tempdir := `mktemp -d`

default: fmt test lint

# taplo walks the tree itself and asks git nothing, so pointing it at the
# repository is how `just fmt` came to rewrite the `Cargo.toml` that `cargo
# package` leaves under `target/`. git is asked instead: `--cached` for the
# tracked ones and `--others --exclude-standard` for a TOML that is written but
# has never been added, which is the one a tracked-only list would miss.
# Mirroring `.gitignore` into `.taplo.toml` was the other way and cannot be
# done — `scripts/benchmarks/data/gitlab/.gitignore` alone runs to some 600
# entries.
#
# `bash` with `pipefail`, which just's `sh -cu` has not: a `git ls-files` that
# failed would otherwise leave `xargs -r` nothing to do and the recipe green
# having formatted nothing. An extracted release tarball has no `.git`, and
# that is what it would have looked like there.
fmt:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo fmt --all --check
    nix fmt flake.nix
    git ls-files -z --cached --others --exclude-standard '*.toml' | xargs -0 -r taplo format

test:
    CLICOLOR_FORCE=true cargo test --locked --all-features --workspace

lint:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo clippy --locked --all-targets --all-features --workspace -- -D warnings
    git ls-files -z --cached --others --exclude-standard '*.toml' | xargs -0 -r taplo lint

cov:
    CLICOLOR_FORCE=true cargo llvm-cov --locked --open

[linux]
flamegraph target="scripts/benchmarks/data/gitlab":
    cargo flamegraph --profile bench --open -- check {{ target }}

[macos]
flamegraph target="scripts/benchmarks/data/gitlab":
    # See https://github.com/flamegraph-rs/flamegraph#dtrace-on-macos
    cargo flamegraph --root --profile bench --open -- check {{ target }}

# `cargo fuzz` takes no `--locked` and forwards no cargo flags that mean it, so
# a run whose manifests have moved rewrites the lock on its way past — the one
# CD builds against and `check-versions.sh` reads, now that `fuzz/` shares it.
# `cargo metadata --locked` is the same question asked first. Not free on a cold
# cache — it extracts every package in the resolve, dev-dependencies a fuzz
# build never touches included — but anyone reaching for `just fuzz` has run
# `just test` before it.
fuzz target="linter":
    cargo metadata --locked --format-version 1 > /dev/null
    cargo +nightly fuzz run {{ target }}

[private]
download-hash version target:
    @echo 'Downloading v{{ version }}/{{ target }}...'
    @wget -q -P {{ tempdir }} https://github.com/akiomik/mado/releases/download/v{{ version }}/{{ target }}
    @mv {{ tempdir }}/{{ target }} {{ tempdir }}/v{{ version }}-{{ target }}

[private]
update-homebrew-hash target: (download-hash prev_version target) (download-hash version target)
    @echo 'Updating pkg/homebrew/mado.rb for {{ target }}...'
    @prev_hash=`cut -d ' ' -f 1 {{ tempdir }}/v{{ prev_version }}-{{ target }}` \
      && new_hash=`cut -d ' ' -f 1 {{ tempdir }}/v{{ version }}-{{ target }}` \
      && sed -I '' "s/$prev_hash/$new_hash/" pkg/homebrew/mado.rb

update-homebrew-hash-linux-arm64: (update-homebrew-hash "mado-Linux-gnu-arm64.tar.gz.sha256")

update-homebrew-hash-linux-amd64: (update-homebrew-hash "mado-Linux-gnu-x86_64.tar.gz.sha256")

update-homebrew-hash-macos-arm64: (update-homebrew-hash "mado-macOS-arm64.tar.gz.sha256")

update-homebrew-hash-macos-amd64: (update-homebrew-hash "mado-macOS-x86_64.tar.gz.sha256")

update-homebrew-hash-all: update-homebrew-hash-linux-arm64 update-homebrew-hash-linux-amd64 update-homebrew-hash-macos-arm64 update-homebrew-hash-macos-amd64

update-homebrew: update-homebrew-hash-all
    @echo 'Updating pkg/homebrew/mado.rb for {{ version }}...'
    @sed -I '' "s/{{ prev_version }}/{{ version }}/" pkg/homebrew/mado.rb

[private]
update-scoop-hash target: (download-hash prev_version target) (download-hash version target)
    @echo 'Updating pkg/scoop/mado.json for {{ target }}...'
    @prev_hash=`cut -d ' ' -f 1 {{ tempdir }}/v{{ prev_version }}-{{ target }}` \
      && new_hash=`cut -d ' ' -f 1 {{ tempdir }}/v{{ version }}-{{ target }}` \
      && sed -I '' "s/$prev_hash/$new_hash/" pkg/scoop/mado.json

update-scoop-hash-windows-amd64: (update-scoop-hash "mado-Windows-msvc-x86_64.zip.sha256")

update-scoop-hash-all: update-scoop-hash-windows-amd64

update-scoop: update-scoop-hash-all
    @echo 'Updating pkg/scoop/mado.json for {{ version }}...'
    @sed -I '' "s/{{ prev_version }}/{{ version }}/" pkg/scoop/mado.json

[private]
update-winget-hash target: (download-hash prev_version target) (download-hash version target)
    @echo 'Updating pkg/winget/mado.yml for {{ target }}...'
    @prev_hash=`cut -d ' ' -f 1 {{ tempdir }}/v{{ prev_version }}-{{ target }}` \
      && new_hash=`cut -d ' ' -f 1 {{ tempdir }}/v{{ version }}-{{ target }}` \
      && sed -I '' "s/$prev_hash/$new_hash/" pkg/winget/mado.yml

update-winget-hash-windows-amd64: (update-winget-hash "mado-Windows-msvc-x86_64.zip.sha256")

update-winget-hash-all: update-winget-hash-windows-amd64

# `ReleaseDate` is the date WinGet shows the installer as released on, so it has
# to move with every version. It comes from the changelog rather than today's
# date: `update-winget` runs once CD has finished, which is not always the day
# the release is dated. Only `YYYY-MM-DD` counts: that is all the manifest
# schema's `date` format admits, and a heading written any other way does not
# say which day it means.
#
# `update-winget` reads the date through this recipe, and checks the manifest is
# the one being replaced and still has a `ReleaseDate` line to stamp, before the
# checksums are downloaded. Whatever cannot be answered stops the recipe while
# the manifest is still untouched, rather than after the version and the
# checksum have gone in.
#
# Checking `PackageVersion` is what keeps the stamp honest. It is the one edit
# in `update-winget` that does not key on `prev_version`, so a `prev_version`
# left behind would leave the version, the URL and the checksum untouched, stamp
# the date anyway, and report a finished release in a one-line diff.
[private]
winget-release-date:
    @echo 'Reading the release date for {{ version }} from CHANGELOG.md...'
    @sed -n 's/^## \[{{ version }}\] - \([0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}\)$/\1/p' CHANGELOG.md \
      > {{ tempdir }}/release-date
    @dates=`wc -l < {{ tempdir }}/release-date` \
      && if [ "$dates" -eq 0 ]; then \
           echo 'CHANGELOG.md has no `## [{{ version }}] - YYYY-MM-DD` section' >&2; \
           exit 1; \
         elif [ "$dates" -gt 1 ]; then \
           echo 'CHANGELOG.md dates `## [{{ version }}]` more than once' >&2; \
           exit 1; \
         fi
    @at=`sed -n 's/^PackageVersion: //p' pkg/winget/mado.yml` \
      && if [ "$at" != '{{ prev_version }}' ]; then \
           echo "pkg/winget/mado.yml is at '$at'; prev_version in the justfile names {{ prev_version }}" >&2; \
           exit 1; \
         fi
    @if ! grep -q '^ReleaseDate: ' pkg/winget/mado.yml; then \
         echo 'pkg/winget/mado.yml has no ReleaseDate line to stamp' >&2; \
         exit 1; \
       fi

update-winget: winget-release-date update-winget-hash-all
    @echo 'Updating pkg/winget/mado.yml for {{ version }}...'
    @sed -I '' "s/{{ prev_version }}/{{ version }}/" pkg/winget/mado.yml
    @release_date=`cat {{ tempdir }}/release-date` \
      && echo "Stamping ReleaseDate as $release_date..." \
      && sed -I '' "s/^ReleaseDate: .*/ReleaseDate: $release_date/" pkg/winget/mado.yml

[private]
nix-hash version target:
    @echo 'Downloading v{{ version }}/{{ target }}...'
    @nix-prefetch-url --unpack https://github.com/akiomik/mado/releases/download/v{{ version }}/{{ target }} \
      > {{ tempdir }}/v{{ version }}-{{ target }}.sha256

[private]
update-flake-hash target: (nix-hash prev_version target) (nix-hash version target)
    @echo 'Updating flake.nix for {{ target }}...'
    @prev_hash=`cat {{ tempdir }}/v{{ prev_version }}-{{ target }}.sha256` \
      && new_hash=`cat {{ tempdir }}/v{{ version }}-{{ target }}.sha256` \
      && sed -I '' "s/$prev_hash/$new_hash/" flake.nix

update-flake-hash-linux-arm64: (update-flake-hash "mado-Linux-gnu-arm64.tar.gz")

update-flake-hash-linux-amd64: (update-flake-hash "mado-Linux-gnu-x86_64.tar.gz")

update-flake-hash-macos-arm64: (update-flake-hash "mado-macOS-arm64.tar.gz")

update-flake-hash-macos-amd64: (update-flake-hash "mado-macOS-x86_64.tar.gz")

update-flake-hash-all: update-flake-hash-linux-arm64 update-flake-hash-linux-amd64 update-flake-hash-macos-arm64 update-flake-hash-macos-amd64

update-flake: update-flake-hash-all
    @echo 'Updating flake.nix for {{ version }}...'
    @sed -I '' "s/{{ prev_version }}/{{ version }}/" flake.nix
