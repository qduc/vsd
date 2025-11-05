#!/bin/bash

set -e

PACKAGES_DIR="$(cd "$(dirname "$0")" && pwd)/packages"

# Detect package manager
if command -v dnf >/dev/null 2>&1; then
    PKG_MGR="dnf"
else
    PKG_MGR="apt"
fi

MACOS_SDK_VERSION="15.4" # https://github.com/joseluisq/macosx-sdks/releases
PROTOC_VERSION="31.1" # https://github.com/protocolbuffers/protobuf/releases
ZIG_VERSION="0.14.1" # https://ziglang.org/download

echo "Installing Build Dependencies"
if [ "$PKG_MGR" = "apt" ]; then
    sudo apt update && sudo apt upgrade -y
    sudo apt install -y zip unzip build-essential libssl-dev pkgconf \
        bzip2 clang cmake cpio git libxml2-dev llvm-dev lzma-dev patch python3 uuid-dev zlib1g-dev xz-utils
else
    sudo dnf update -y
    sudo dnf install -y zip unzip @development-tools openssl-devel pkgconf-pkg-config \
        bzip2 clang cmake cpio git libxml2-devel llvm-devel xz-devel patch python3 uuid-devel zlib-devel xz
fi

rm -rf $PACKAGES_DIR
mkdir -p $PACKAGES_DIR

if ! command -v rustc >/dev/null 2>&1; then
  echo "Installing Rust"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  . "$HOME/.cargo/env"
fi

rustup target add \
  aarch64-apple-darwin \
  aarch64-linux-android \
  aarch64-pc-windows-msvc \
  aarch64-unknown-linux-musl \
  x86_64-apple-darwin \
  x86_64-pc-windows-msvc \
  x86_64-unknown-linux-musl

echo "Installing cargo-zigbuild"
cargo install cargo-zigbuild

echo "Installing cargo-xwin"
cargo install cargo-xwin

echo "Installing Protoc v$PROTOC_VERSION"
curl -L https://github.com/protocolbuffers/protobuf/releases/download/v$PROTOC_VERSION/protoc-$PROTOC_VERSION-linux-x86_64.zip -o protoc-$PROTOC_VERSION-linux-x86_64.zip
unzip protoc-$PROTOC_VERSION-linux-x86_64.zip -d $PACKAGES_DIR/protoc-$PROTOC_VERSION
rm protoc-$PROTOC_VERSION-linux-x86_64.zip

echo "Installing Zig v$ZIG_VERSION"
curl -L https://ziglang.org/download/0.14.1/zig-x86_64-linux-0.14.1.tar.xz | tar xJC $PACKAGES_DIR

echo "Installing Osxcross"
git clone https://github.com/tpoechtrager/osxcross $PACKAGES_DIR/osxcross
curl -L https://github.com/joseluisq/macosx-sdks/releases/download/$MACOS_SDK_VERSION/MacOSX$MACOS_SDK_VERSION.sdk.tar.xz -o $PACKAGES_DIR/osxcross/tarballs/MacOSX$MACOS_SDK_VERSION.sdk.tar.xz
cd "$PACKAGES_DIR/osxcross" || exit 1

echo "Build dependencies installed successfully."
