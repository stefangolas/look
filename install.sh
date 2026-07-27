#!/usr/bin/env sh
set -eu

repo="${LOOK_REPOSITORY:-stefangolas/look}"
version="${LOOK_VERSION:-latest}"
install_dir="${LOOK_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
  Linux) os=linux ;;
  Darwin) os=macos ;;
  *) echo "look: unsupported operating system: $(uname -s)" >&2; exit 1 ;;
esac
case "$(uname -m)" in
  x86_64|amd64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) echo "look: unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

if [ "$version" = latest ]; then
  version=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$repo/releases/latest" | sed 's#.*/##')
fi
asset="look-${version}-${os}-${arch}.tar.gz"
base="https://github.com/$repo/releases/download/${version}"
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT INT TERM
curl -fL --retry 3 -o "$temporary/$asset" "$base/$asset"
curl -fL --retry 3 -o "$temporary/$asset.sha256" "$base/$asset.sha256"
(cd "$temporary" && (sha256sum -c "$asset.sha256" 2>/dev/null || shasum -a 256 -c "$asset.sha256"))
tar -xzf "$temporary/$asset" -C "$temporary"
mkdir -p "$install_dir"
cp "$temporary/look-${version}-${os}-${arch}/look" "$install_dir/look"
chmod 755 "$install_dir/look"
echo "look installed to $install_dir/look"
case ":$PATH:" in
  *":$install_dir:"*) ;;
  *) echo "Add $install_dir to PATH to invoke look from any shell." ;;
esac
