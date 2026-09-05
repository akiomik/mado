#!/bin/sh

# `CDPATH=` because `cd` resolves a relative operand through it and echoes
# where it landed, which would end up in this substitution.
# shellcheck disable=SC1007  # `CDPATH=` is the point, not a typo
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd) || exit 1
DATA_ROOT=$SCRIPT_DIR/data
CLONE_DIR=$DATA_ROOT/markdownlint

# The only tool this script needs, asked for the way `test.sh` and
# `comparison.sh` ask for theirs: absent, the clone below fails and the `cd`
# after it points at a directory rather than at the missing `git`.
if ! command -v git > /dev/null; then
  echo "git is not installed" >&2
  exit 1
fi

cd "$DATA_ROOT" || exit 1
git clone --sparse --filter=blob:none https://github.com/markdownlint/markdownlint.git
# Everything below takes the repository the working directory belongs to, so
# arriving here matters, and by absolute path: a relative one goes through
# `CDPATH`. A directory that exists without being a repository still passes
# this and sends those commands into mado itself; that is #399.
cd "$CLONE_DIR" || exit 1
# `cd` proves the directory is there, not that it is the clone, and git resolves
# `.git` upward: against a directory that exists without being a repository,
# the commands below reach mado's own and `sparse-checkout set` empties its
# working tree. Testing for the directory, for a `.git` inside it, and setting
# `GIT_CEILING_DIRECTORIES` all failed to catch that; #399 has why.
#
# Asking for the prefix rather than comparing paths: it is empty only when the
# working directory *is* a repository's top level, and comparing
# `--show-toplevel` against `pwd -P` would call a good clone bad wherever the
# two spell the same directory differently, as Git for Windows does.
if ! prefix=$(git rev-parse --show-prefix 2> /dev/null) || [ -n "$prefix" ]; then
  echo "$CLONE_DIR is not a git repository; remove it and run this again" >&2
  exit 1
fi

git sparse-checkout set test/rule_tests || exit 1
git checkout || exit 1

# Set aside the fixtures that cannot be compared, leaving the ones both tools
# can be pointed at.
#
# Upstream pairs some fixtures with an `X_style.rb`, and `test/test_rules.rb`
# loads it to select the rules that fixture is checked against and to set their
# parameters: `long_lines_100_style.rb` is `rule 'MD013', :line_length => 100`
# beside a document written to that width. `test.sh` cannot reproduce it: it
# runs each tool once, mdl under `.mdlrc` and mado under the `mado.toml` beside
# it, and neither of those is the fixture's own style. The document would be
# read at whatever width those two say, so comparing it there measures the
# fixture's style rather than the two tools.
#
# TODO: apply the per-fixture styles instead of skipping them, and drop this.

# Relative to the clone `cd`ed into above, not to the repository root.
# `default_test_style.rb` is the fallback `test_rules.rb` uses for every fixture
# without a style of its own, so it names no document.
#
# Handed to `find -exec` rather than iterated as a word list: an unquoted
# expansion is not split at all under zsh, which would make the whole list one
# iteration. `test.sh` reads the corpus the same way.
# The body below is keyed on the document, not on the `.bak`: those are
# untracked, so a `git restore` in the clone brings the documents back and
# leaves them, and keying on them would skip every fixture in silence.
#
# Neither present means it derives no document from that style file. Not fatal,
# there being nothing in the corpus to have left there, and upstream may add a
# second shared style file beside the fallback.
#
# Written out here rather than inside the single-quoted body, where one
# apostrophe in a later edit would close the quote and hand `find` a different
# operand list.
set_aside=$(
  find test/rule_tests -name '*_style.rb' ! -name 'default_test_style.rb' \
    -exec sh -c 'for style_file in "$@"; do
      markdown_file=${style_file%_style.rb}.md

      if [ -f "$markdown_file" ]; then
        mv "$markdown_file" "$markdown_file.bak" || exit 1
        printf .
      elif [ -f "$markdown_file.bak" ]; then
        printf .
      else
        echo "no document for $style_file" >&2
      fi
    done' _ {} +
) || exit 1

# Having set nothing aside must not pass for having done the job: a corpus with
# no fixtures in it, or one holding only the fallback style file, would
# otherwise exit 0.
if [ -z "$set_aside" ]; then
  echo "set nothing aside under $(pwd)/test/rule_tests" >&2
  echo "if the documents are gone too, restore them with: git restore ." >&2
  exit 1
fi

# Said out loud because the dots above are the count and are captured rather
# than printed. What a re-run does print (`git clone` reporting the directory
# already exists, and `git checkout` listing every set-aside document as
# deleted) reads like a run that went wrong.
#
# A statement about the corpus, not about this run: the count includes the
# fixtures a previous run set aside, which this one had nothing to do for.
echo "$(printf %s "$set_aside" | wc -c | tr -d ' ') fixtures are set aside" >&2
