#!/bin/sh

set -eu

repository="TheZero0-ctrl/agentmux"
install_dir="${AGENTMUX_INSTALL_DIR:-${HOME}/.local/bin}"
version="${1:-latest}"

for command in curl grep install mktemp sha256sum tar; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'error: required command not found: %s\n' "$command" >&2
    exit 1
  fi
done

if [ "$(uname -s)" != "Linux" ]; then
  printf 'error: agentmux releases currently support Linux only\n' >&2
  exit 1
fi

case "$(uname -m)" in
  x86_64 | amd64) target="x86_64-unknown-linux-gnu" ;;
  *)
    printf 'error: agentmux releases currently support x86_64 only\n' >&2
    exit 1
    ;;
esac

if [ "$version" = "latest" ]; then
  latest_url=$(curl -fsSL -o /dev/null -w '%{url_effective}' \
    "https://github.com/${repository}/releases/latest")
  version=${latest_url##*/}
else
  case "$version" in
    v*) ;;
    *) version="v${version}" ;;
  esac
fi

if ! printf '%s\n' "$version" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
  printf 'error: invalid release version: %s\n' "$version" >&2
  exit 1
fi

archive="agentmux-${version}-${target}.tar.gz"
download_url="https://github.com/${repository}/releases/download/${version}"
temporary_dir=$(mktemp -d)
trap 'rm -rf "$temporary_dir"' EXIT HUP INT TERM

printf 'Downloading agentmux %s...\n' "$version"
curl -fsSL "${download_url}/${archive}" -o "${temporary_dir}/${archive}"
curl -fsSL "${download_url}/${archive}.sha256" -o "${temporary_dir}/${archive}.sha256"

expected=$(while IFS=' ' read -r digest remainder; do printf '%s' "$digest"; break; done < "${temporary_dir}/${archive}.sha256")
actual=$(sha256sum "${temporary_dir}/${archive}")
actual=${actual%% *}

if [ -z "$expected" ] || [ "$actual" != "$expected" ]; then
  printf 'error: checksum verification failed\n' >&2
  exit 1
fi

tar -xzf "${temporary_dir}/${archive}" -C "$temporary_dir"
install -d "$install_dir"
install -m 755 "${temporary_dir}/agentmux" "${install_dir}/agentmux"

printf 'Installed agentmux to %s/agentmux\n' "$install_dir"
case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *) printf 'Add %s to your PATH to run agentmux.\n' "$install_dir" ;;
esac
