#!/bin/sh

# shellcheck disable=SC1007  # `CDPATH=` is the point, not a typo
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd) || exit 1
PROJECT_ROOT=$SCRIPT_DIR/../..
DATA_ROOT=$SCRIPT_DIR/data
# Where cargo will have put the binary. `CARGO_TARGET_DIR` moves the root and
# `CARGO_BUILD_TARGET` adds a triple below it; `build.target-dir` and
# `build.target` in a cargo config do the same and cannot be seen from here.
TARGET_DIR=${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}
if [ -n "$CARGO_BUILD_TARGET" ]; then
  TARGET_DIR=$TARGET_DIR/$CARGO_BUILD_TARGET
fi
DOC_PATH=$DATA_ROOT/gitlab/doc

# Named once, so the guards below, the smoke test and the commands hyperfine
# runs cannot drift apart.
MADO_BIN=$TARGET_DIR/release/mado
MADO_CONFIG=$SCRIPT_DIR/mado.toml
MDL_CONFIG=$SCRIPT_DIR/.mdlrc
MARKDOWNLINT_BIN=$SCRIPT_DIR/node_modules/.bin/markdownlint
MARKDOWNLINT_CLI2_BIN=$SCRIPT_DIR/node_modules/.bin/markdownlint-cli2
MARKDOWNLINT_CONFIG=$SCRIPT_DIR/.markdownlint.jsonc

# `cargo build` finds its project through the working directory, and hyperfine
# runs with `--ignore-failure`, so from anywhere else this would build nothing
# and then time whatever stale binary is on disk, or a `command not found` at
# microsecond speed, and report either as a result.
cd "$PROJECT_ROOT" || exit 1

# Every command hyperfine is about to time, asked for before any of them runs.
# A missing one is not an error to hyperfine: it times the shell failing to
# start it, in microseconds, and prints that beside mado's real number. Nothing
# installs `node_modules` here, so those two are absent on a fresh clone rather
# than exceptionally.
for tool in hyperfine mdl node cargo; do
  if ! command -v "$tool" > /dev/null; then
    echo "$tool is not installed" >&2
    exit 1
  fi
done

# `node` among them because both of these are `#!/usr/bin/env node` wrappers,
# so `-x` says they can be started while they cannot run. A `node_modules`
# restored from a cache rather than installed is where that shows up.
#
# `-x` rather than `command -v`, which is only exact for a name looked up on
# `PATH`: given a path, dash reports an existing file as found whether or not
# it can be run, so an unpacked-but-not-executable one would clear the check
# and hyperfine would time "permission denied" instead.
for tool in "$MARKDOWNLINT_BIN" "$MARKDOWNLINT_CLI2_BIN"; do
  if [ ! -x "$tool" ]; then
    echo "$tool is not executable; run: npm --prefix \"$SCRIPT_DIR\" ci" >&2
    exit 1
  fi
done

# What they are given, on the same reasoning: hyperfine cannot tell a tool that
# refused to start from one that ran. `$DOC_PATH` comes from `setup.sh`, which
# nothing here invokes.
# Present but empty counts as absent here: none of `setup.sh`'s git commands is
# checked, so a sparse-checkout that resolved to nothing leaves the directory
# there, and hyperfine would time four tools linting no files.
if [ ! -d "$DOC_PATH" ]; then
  echo "$DOC_PATH does not exist; run setup.sh first" >&2
  exit 1
fi

# Below that test, not above it: on a fresh clone `find` would otherwise fail
# on the missing directory first and report it in its own words.
#
# `-quit` rather than `| head -n 1`, which would hide a `find` that failed
# outright behind the same message as an empty corpus, and make GNU findutils
# report a broken pipe on a corpus this size. It is a GNU and BSD extension,
# which is the whole of what runs a script needing cargo, node and a Ruby gem.
first_document=$(find "$DOC_PATH" -name '*.md' -print -quit) || exit 1

if [ -z "$first_document" ]; then
  echo "$DOC_PATH holds no documents; run setup.sh first" >&2
  exit 1
fi

for config_file in "$MADO_CONFIG" "$MDL_CONFIG" "$MARKDOWNLINT_CONFIG"; do
  if [ ! -f "$config_file" ]; then
    echo "$config_file does not exist" >&2
    exit 1
  fi
done

# Unchecked, a compile error leaves the previous `target/release/mado` in place
# for hyperfine to time and report as this tree's number.
cargo build --release || exit 1

# And that it landed where hyperfine will look: `CARGO_TARGET_DIR`, a
# `build.target-dir` in the cargo config, or a configured target triple all put
# it elsewhere, and a build that succeeded says nothing about that. Everything
# else timed below is checked; the thing under test should be too.
if [ ! -x "$MADO_BIN" ]; then
  echo "no mado at $MADO_BIN" >&2
  echo "a cargo config's build.target-dir or build.target moves it, and this" >&2
  echo "reads only CARGO_TARGET_DIR and CARGO_BUILD_TARGET" >&2
  exit 1
fi

# Started once each, because being present and executable is not the same as
# working: a `node_modules` restored from a stale cache leaves the wrappers
# runnable and `node` installed, and they still die on `MODULE_NOT_FOUND` in
# milliseconds, which `--ignore-failure` would print as the fastest tool here.
# `--version` is enough to make each resolve itself.
#
# It does not cover a config a tool starts up and then rejects; that is #400.
for tool in "$MADO_BIN" mdl "$MARKDOWNLINT_BIN"; do
  if ! "$tool" --version > /dev/null 2>&1; then
    echo "$tool does not run" >&2
    exit 1
  fi
done

# `markdownlint-cli2` has no version flag; it reads every argument as a glob,
# so `--version` would only pass by being treated as one. A pattern that
# matches nothing is the same no-op, said on purpose. It reads its config
# on the way, which the other three probes do not: `markdownlint` accepts a
# broken one without complaint when nothing matches, so that half stays #400.
if ! "$MARKDOWNLINT_CLI2_BIN" --config "$MARKDOWNLINT_CONFIG" \
  'zz-no-such-file-*.md' > /dev/null 2>&1; then
  echo "$MARKDOWNLINT_CLI2_BIN does not run" >&2
  exit 1
fi

# Passed through the environment rather than pasted into the command strings.
# hyperfine hands each string to a shell, so a path with a space in it has to
# be quoted somehow, and quoting it here would only move the problem to
# apostrophes: a checkout under `/Users/o'brien` would close the quote early.
# Naming the values instead leaves the quoting to the shell that expands them.
#
# Without this the commands fail to start and `--ignore-failure` reports that
# as a sub-millisecond timing beside the real numbers.
export MADO_BIN MADO_CONFIG MDL_CONFIG DOC_PATH \
  MARKDOWNLINT_BIN MARKDOWNLINT_CLI2_BIN MARKDOWNLINT_CONFIG

# Named, because the command strings are now variable references and hyperfine
# labels its results with them: without these the table says
# `"$MADO_BIN" --config …` for every run, and two runs against different
# binaries print identical labels.
hyperfine --ignore-failure --warmup 10 \
  -n mado '"$MADO_BIN" --config "$MADO_CONFIG" check "$DOC_PATH"' \
  -n mdl 'mdl --config "$MDL_CONFIG" "$DOC_PATH"' \
  -n markdownlint '"$MARKDOWNLINT_BIN" --config "$MARKDOWNLINT_CONFIG" "$DOC_PATH"' \
  -n markdownlint-cli2 \
  '"$MARKDOWNLINT_CLI2_BIN" --config "$MARKDOWNLINT_CONFIG" "$DOC_PATH/**/*.md"'
