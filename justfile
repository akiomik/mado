prev_version := "0.3.1"
version := "0.3.2"
tempdir := `mktemp -d`

default: fmt test lint

fmt:
    cargo fmt --all --check
    nix fmt flake.nix
    taplo format

test:
    CLICOLOR_FORCE=true cargo test --locked --all-features --workspace

lint:
    cargo clippy --locked --all-targets --all-features --workspace -- -D warnings
    taplo lint

cov:
    CLICOLOR_FORCE=true cargo llvm-cov --locked --open

[linux]
flamegraph target="scripts/benchmarks/data/gitlab":
    cargo flamegraph --profile bench --open -- check {{ target }}

[macos]
flamegraph target="scripts/benchmarks/data/gitlab":
    # See https://github.com/flamegraph-rs/flamegraph#dtrace-on-macos
    cargo flamegraph --root --profile bench --open -- check {{ target }}

# Fuzz a target. `cargo fuzz` takes no `--locked`; `cargo metadata` asks first.
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
