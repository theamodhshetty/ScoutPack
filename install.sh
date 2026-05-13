#!/usr/bin/env sh
set -eu

REPO="theamodhshetty/ScoutPack"
VERSION="${SCOUTPACK_VERSION:-latest}"
INSTALL_DIR="${SCOUTPACK_INSTALL_DIR:-}"

usage() {
  cat <<'USAGE'
Install ScoutPack from GitHub Releases.

Usage:
  install.sh [--version v0.1.0] [--dir /usr/local/bin]

Environment:
  SCOUTPACK_VERSION       Release tag. Default: latest
  SCOUTPACK_INSTALL_DIR   Install directory. Default: /usr/local/bin if writable, else ~/.local/bin

Examples:
  curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh | sh
  curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh | sh -s -- --dir "$HOME/.local/bin"
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      VERSION="${2:?missing version}"
      shift 2
      ;;
    --dir)
      INSTALL_DIR="${2:?missing install dir}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "required command not found: $1" >&2
    exit 1
  fi
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux) os_part="unknown-linux-gnu" ;;
    Darwin) os_part="apple-darwin" ;;
    *)
      echo "unsupported OS for install.sh: $os" >&2
      echo "Windows users should use the release zip or Scoop manifest." >&2
      exit 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64) arch_part="x86_64" ;;
    arm64|aarch64) arch_part="aarch64" ;;
    *)
      echo "unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac

  printf '%s-%s\n' "$arch_part" "$os_part"
}

download() {
  url="$1"
  output="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$output"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$url" -O "$output"
  else
    echo "required command not found: curl or wget" >&2
    exit 1
  fi
}

default_install_dir() {
  if [ -w /usr/local/bin ]; then
    printf '%s\n' /usr/local/bin
  else
    printf '%s\n' "$HOME/.local/bin"
  fi
}

main() {
  need_cmd uname
  need_cmd tar
  need_cmd mktemp

  target="$(detect_target)"
  archive="scoutpack-${target}.tar.gz"

  if [ "$VERSION" = "latest" ]; then
    url="https://github.com/${REPO}/releases/latest/download/${archive}"
  else
    url="https://github.com/${REPO}/releases/download/${VERSION}/${archive}"
  fi

  if [ -z "$INSTALL_DIR" ]; then
    INSTALL_DIR="$(default_install_dir)"
  fi

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' EXIT INT HUP

  echo "Downloading $url"
  download "$url" "$tmp_dir/$archive"

  tar -xzf "$tmp_dir/$archive" -C "$tmp_dir"
  mkdir -p "$INSTALL_DIR"
  cp "$tmp_dir/scoutpack-${target}/scoutpack" "$INSTALL_DIR/scoutpack"
  chmod 755 "$INSTALL_DIR/scoutpack"

  echo "Installed ScoutPack to $INSTALL_DIR/scoutpack"
  "$INSTALL_DIR/scoutpack" --version
}

main "$@"
