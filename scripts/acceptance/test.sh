#!/bin/sh
#
# Writes what mdl and mado each report for the acceptance corpus, to
# `tmp/mdl.txt` and `tmp/mado.txt`. Comparing them is the caller's to do; this
# says nothing about whether they agree.

# shellcheck disable=SC1007  # `CDPATH=` is the point, not a typo
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd) || exit 1
PROJECT_ROOT=$SCRIPT_DIR/../..
DATA_ROOT=$SCRIPT_DIR/data
DOC_PATH=$DATA_ROOT/markdownlint/test/rule_tests
TEMP_PATH=$PROJECT_ROOT/tmp
# Named once, so the guard below and the runs further down cannot drift apart:
# checking a file the run does not use is the failure this guards against.
MDL_CONFIG=$SCRIPT_DIR/.mdlrc
MADO_CONFIG=$SCRIPT_DIR/../benchmarks/mado.toml

# `tmp/` at the repository root is gitignored and nothing else creates it, so
# on a fresh clone both redirects below fail and the `sed`s run against files
# that are not there. A redirection that fails does not end a non-interactive
# shell, so an unchecked `mkdir` would land back in exactly that state.
mkdir -p "$TEMP_PATH" || exit 1

# Before the guards, so that stopping at one of them leaves no pair behind. A
# previous run's would otherwise sit there looking finished and be read as this
# one's. That does cost a good pair when a precondition fails, which is the
# trade taken deliberately: these two are regenerable by re-running, and being
# misled by them is not recoverable in the same way.
#
# Everything below writes to working names and moves both into place at the
# end, which covers the same ground for the steps after the guards.
rm -f "$TEMP_PATH/mdl.txt" "$TEMP_PATH/mado.txt"

# The working names, on every exit between here and the final rename, which
# would otherwise leave some of them to sit until the next run.
#
# The second trap is not redundant: an `EXIT` trap runs when a signal is caught
# but not when the shell dies from an uncaught one, and `dash` and `zsh` both
# leave the debris on a `TERM`, a CI cancelling a job. Exiting from the
# signal handler is what reaches the cleanup.
trap 'rm -f "$TEMP_PATH/mdl.part" "$TEMP_PATH/mado.part" \
  "$TEMP_PATH/mdl.done" "$TEMP_PATH/mado.done"
  [ -f "$TEMP_PATH/mdl.txt" ] && [ -f "$TEMP_PATH/mado.txt" ] ||
    rm -f "$TEMP_PATH/mdl.txt" "$TEMP_PATH/mado.txt"' EXIT
trap 'exit 1' INT TERM HUP

# Ahead of the build, all of these being cheap and none a precondition for it.
# Each is separable from what the tools reported, which is deliberately not
# checked: both exit non-zero when they find violations, the normal case here.
#
# `mdl` is a Ruby gem that nothing here installs, and an absent one leaves the
# redirect's empty `mdl.txt` reading as mdl having found nothing. `cargo` fails
# loudly rather than quietly, but a contributor without a Rust toolchain should
# hear about it here rather than after the other guards have run.
for tool in mdl cargo; do
  if ! command -v "$tool" > /dev/null; then
    echo "$tool is not installed" >&2
    exit 1
  fi
done

# The two configs themselves, which this script now names rather than leaves to
# discovery. `mado --config` on a file that is not there exits 1 with nothing on
# stdout, indistinguishable from finding violations, and `mdl` exits 3 the same
# way; either leaves an empty file reading as that tool having found nothing.
# `.mdlrc` is a symlink into `../benchmarks`, so this catches a broken one too.
for config_file in "$MDL_CONFIG" "$MADO_CONFIG"; do
  if [ ! -f "$config_file" ]; then
    echo "$config_file does not exist" >&2
    exit 1
  fi
done

# `mado check` on a path that is not there exits 0, saying only "All checks
# passed!" on stdout and putting the reason on stderr, and `mdl` prints nothing
# at all, so this would otherwise compare two files with no findings in them.
if [ ! -d "$DOC_PATH" ]; then
  echo "$DOC_PATH does not exist; run setup.sh first" >&2
  exit 1
fi

# And that `setup.sh` did its half of the job, not merely that it ran once.
# Searched recursively, as `setup.sh` searches, so the two cannot disagree
# about what the corpus holds if it ever stops being flat.
style_files=$(find "$DOC_PATH" -name '*_style.rb') || exit 1
documents=$(find "$DOC_PATH" -name '*.md') || exit 1

# A corpus with nothing in it lints clean, which is the same "answer without
# having run anything" as the cases above.
if [ -z "$style_files" ] || [ -z "$documents" ]; then
  echo "no fixtures in $DOC_PATH; run setup.sh" >&2
  exit 1
fi

# What `setup.sh` leaves behind is the absence of a document beside any style
# file, so that is what to look for. Not the `.bak` files: those are untracked,
# and a `git restore` inside the clone brings the documents back while leaving
# every one of them in place. Comparing those under one shared config is the
# wrong answer rather than no answer.
# Collected rather than piped into a loop. `$DOC_PATH` is absolute, so every
# result carries the checkout's own path and a space in it would split the list
# into pieces that exist nowhere; and an `exit` in the last stage of a pipeline
# ends the whole script on the shells that run that stage in the current one,
# taking the message below with it.
leftover=$(
  find "$DOC_PATH" -name '*_style.rb' ! -name 'default_test_style.rb' \
    -exec sh -c 'for f; do
      if [ -f "${f%_style.rb}.md" ]; then printf "%s\n" "$f"; fi
    done' _ {} +
) || exit 1

if [ -n "$leftover" ]; then
  echo "fixtures are not set aside in $DOC_PATH; run setup.sh" >&2
  exit 1
fi

# `cargo run` finds its project through the working directory, so run from
# anywhere else it exits 101 into an already-created, empty `mado.txt` that
# compares as mado having found nothing.
cd "$PROJECT_ROOT" || exit 1

# A failed build is that same empty file, and `cargo run`'s status cannot be
# told apart from mado's.
cargo build || exit 1


# Not gated on what they found, both returning 1 for violations, the normal
# case; but anything above that is the tool failing, and the redirect has already
# truncated the file it would have written to. mado buffers and writes at the
# end, so a panic leaves an empty one that reads as having found nothing.
mdl --config "$MDL_CONFIG" "$DOC_PATH" > "$TEMP_PATH/mdl.part"
rc=$?
if [ "$rc" -gt 1 ]; then
  echo "mdl exited $rc" >&2
  exit 1
fi

# And not merely that it exited politely: mdl says nothing at all when its
# config names a rule this version does not implement, exiting 0 as it does so.
# It also says nothing when the corpus is clean, which cannot be told apart
# from here, and either way there is nothing to compare, so stopping is right
# and the message names both.
if [ ! -s "$TEMP_PATH/mdl.part" ]; then
  echo "mdl reported nothing: a clean corpus, or a rule it does not know" >&2
  exit 1
fi

# Named, rather than left to whichever `mado.toml` the working directory turns
# up. The repository's own is mado's self-lint config and reports differently
# from what `.mdlrc` asks of mdl: `allow-different-nesting = true` there
# hides MD024 divergences this corpus exists to surface. `../benchmarks` holds
# the counterpart: the same `.mdlrc` (this one is a symlink to it) beside the
# `mado.toml` written to go with it. Not a perfect match either: its
# `[lint.md007] indent = 4` is mdl's 3, which is most of what the two still
# disagree about, but it is the pair, and #401 is where that is settled.
#
# `< /dev/null` because mado lints stdin instead of the paths given when stdin
# is neither a terminal nor empty. An empty one falls through to the paths, so
# this is about content: piped so much as a newline, mado would report on that
# and call the corpus clean.
#
# `--` because `--config` means something to cargo too, and mado takes it
# before the subcommand.
cargo run -- --config "$MADO_CONFIG" \
  check --output-format=mdl "$DOC_PATH" > "$TEMP_PATH/mado.part" < /dev/null
rc=$?
if [ "$rc" -gt 1 ]; then
  echo "mado exited $rc" >&2
  exit 1
fi

# A malformed config is the case this catches: mado exits 1 with nothing on
# stdout, which is what finding violations looks like.
if [ ! -s "$TEMP_PATH/mado.part" ]; then
  echo "mado wrote nothing" >&2
  exit 1
fi

# Truncate unnecessary texts.
#
# Reading one name and writing another rather than `sed -i ''`: the empty
# suffix is BSD's spelling, and GNU sed reads it as another input file, so the
# same line prints `can't read :` and exits non-zero having edited the file
# correctly. This form works the same everywhere and can be checked.
sed -e "/^Further documentation is available for these failures:/d" \
  -e "/^ - /d" "$TEMP_PATH/mdl.part" > "$TEMP_PATH/mdl.done" || exit 1
sed -e "/^Found /d" "$TEMP_PATH/mado.part" > "$TEMP_PATH/mado.done" || exit 1

# Both, and only once both are ready. The trap asks whether the pair is there
# rather than whether the run reached a flag, so there is no window in which a
# signal can catch it: after the first rename only one exists and both go, and
# after the second both exist and both stay.
mv "$TEMP_PATH/mdl.done" "$TEMP_PATH/mdl.txt" || exit 1
mv "$TEMP_PATH/mado.done" "$TEMP_PATH/mado.txt" || exit 1
