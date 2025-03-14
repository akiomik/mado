#!/bin/sh

SCRIPT_DIR=$(cd $(dirname $0); pwd)
PROJECT_ROOT=($SCRIPT_DIR/../..)
DATA_ROOT=$SCRIPT_DIR/data
DOC_PATH=$DATA_ROOT/gitlab/doc

cargo build --release

hyperfine --ignore-failure --warmup 10 \
  "$PROJECT_ROOT/target/release/mado --config $SCRIPT_DIR/mado.toml check $DOC_PATH" \
  "mdl --config $SCRIPT_DIR/.mdlrc $DOC_PATH" \
  "$SCRIPT_DIR/node_modules/.bin/markdownlint --config $SCRIPT_DIR/.markdownlint.jsonc $DOC_PATH" \
  "$SCRIPT_DIR/node_modules/.bin/markdownlint-cli2 --config $SCRIPT_DIR/.markdownlint.jsonc \"$DOC_PATH/**/*.md\""
