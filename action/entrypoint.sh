#!/bin/bash

COMMAND="mado"
VERSION="v0.3.1"
INSTALL_DIR="$HOME/bin"
COMMAND_PATH="$INSTALL_DIR/$COMMAND"

if [[ ! -x "$COMMAND" ]]; then
  ARCH=$(uname -m)
  UNAME=$(uname -s)
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
