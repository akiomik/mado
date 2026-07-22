prev_version := "0.3.0"
version := "0.3.1"
tempdir := `mktemp -d`

default: fmt test lint

fmt:
    cargo fmt --all --check
    nix fmt flake.nix
    taplo format

test:
    CLICOLOR_FORCE=true cargo test --all-features --workspace

lint:
    cargo clippy --all-targets --all-features --workspace -- -D warnings
    taplo lint

cov:
    CLICOLOR_FORCE=true cargo llvm-cov --open

[linux]
flamegraph target="scripts/benchmarks/data/gitlab":
    cargo flamegraph --profile bench --open -- check {{ target }}

[macos]
flamegraph target="scripts/benchmarks/data/gitlab":
    # See https://github.com/flamegraph-rs/flamegraph#dtrace-on-macos
    cargo flamegraph --root --profile bench --open -- check {{ target }}

fuzz target="linter":
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

update-winget: update-winget-hash-all
    @echo 'Updating pkg/winget/mado.yml for {{ version }}...'
    @sed -I '' "s/{{ prev_version }}/{{ version }}/" pkg/winget/mado.yml

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
