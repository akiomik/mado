#!/bin/bash

COMMAND="mado"
# The release published alongside this version of the action. The `version`
# input overrides it, because a version bump names its release before the tag
# that creates it exists.
DEFAULT_VERSION="v0.3.1"
VERSION="${INPUT_VERSION:-$DEFAULT_VERSION}"
# This ends up in a path and a URL, and a workflow can wire the input to
# anything, so accept only something shaped like one of our tags.
if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
  echo "'$VERSION' is not a mado release tag" >&2
  exit 1
fi
INSTALL_DIR="$HOME/bin"
COMMAND_PATH="$INSTALL_DIR/$COMMAND"

if [[ ! -x "$COMMAND" ]]; then
  ARCH=$(uname -m)
  UNAME=$(uname -s)
  # Linux reports `aarch64` where the release names the same architecture
  # `arm64`; macOS reports `arm64` already. The asset names come from
  # `houseabsolute/actions-rust-release` in `cd.yml`, so this mapping follows
  # what that action publishes, not what `cd.yml` calls the platform.
  if [[ "$ARCH" == "aarch64" ]]; then
    ARCH="arm64"
  fi
  if [[ "$UNAME" == "Darwin" ]]; then
    DOWNLOAD_FILE="mado-macOS-$ARCH.tar.gz"
  elif [[ "$UNAME" == CYGWIN* || "$UNAME" == MINGW* || "$UNAME" == MSYS* ]]; then
    DOWNLOAD_FILE="mado-Windows-msvc-$ARCH.zip"
  else
    DOWNLOAD_FILE="mado-Linux-gnu-$ARCH.tar.gz"
  fi

  echo "Downloading '$COMMAND' $VERSION"
  wget  --progress=dot:mega "https://github.com/akiomik/mado/releases/download/$VERSION/$DOWNLOAD_FILE"

  mkdir -p $INSTALL_DIR
  if [[ "$UNAME" == CYGWIN* || "$UNAME" == MINGW* || "$UNAME" == MSYS* ]] ; then
    unzip -o $DOWNLOAD_FILE -d $INSTALL_DIR "$COMMAND.exe"
  else
    tar -xvf $DOWNLOAD_FILE -C $INSTALL_DIR $COMMAND
  fi

  rm $DOWNLOAD_FILE
fi

echo "Run '$COMMAND_PATH $INPUT_ARGS'"
$COMMAND_PATH $INPUT_ARGS
