# Installation

## Release installer

Linux and macOS select the operating system and CPU architecture, download the
matching archive and checksum, verify SHA-256, and install to `~/.local/bin`:

```sh
curl -fsSL https://raw.githubusercontent.com/stefangolas/look/main/install.sh | sh
```

Pin a release or choose another destination without modifying the script:

```sh
curl -fsSL https://raw.githubusercontent.com/stefangolas/look/main/install.sh |
  LOOK_VERSION=v0.1.0 LOOK_INSTALL_DIR=/usr/local/bin sh
```

Windows PowerShell installs to `%LOCALAPPDATA%\Programs\look\bin` and adds that
directory to the user PATH:

```powershell
irm https://raw.githubusercontent.com/stefangolas/look/main/install.ps1 | iex
```

To pin a version:

```powershell
$env:LOOK_VERSION = 'v0.1.0'
irm https://raw.githubusercontent.com/stefangolas/look/main/install.ps1 | iex
```

The unauthenticated one-line installers require a public GitHub repository and
release. For a private repository, download an authenticated release archive or
build from source.

## Release artifacts

Every tagged release builds and smoke-tests:

- Linux x86-64 and ARM64: `.tar.gz` and `.deb`
- Windows x86-64 and ARM64: `.zip`
- macOS Intel and Apple silicon: `.tar.gz`
- a `.sha256` file for every package

macOS artifacts are ad-hoc signed so their contents are internally sealed. They
are not yet Apple-notarized. Windows artifacts are not yet Authenticode-signed.

Install a Debian package downloaded from a release:

```console
sudo apt install ./look_0.1.0_amd64.deb
```

This is a local package installation, not an APT repository. Publishing a
signed APT repository is a separate distribution step.

## Build from source

Install a stable Rust toolchain and a platform GPU driver that supports one of
the `wgpu` backends, then run:

```console
git clone https://github.com/stefangolas/look.git
cd look
cargo build --locked --release
```

The executable is `target/release/look` or `target\release\look.exe`.

## Cache and local state

`look inspect` stores small, content-validated metadata records containing the
source hash and geometry statistics. It does not currently persist decoded
geometry or GPU buffers to disk.

Default locations:

- Windows: `%LOCALAPPDATA%\look\cache`
- Linux: `$XDG_CACHE_HOME/look/cache` or `~/.cache/look/cache`
- macOS: `$XDG_CACHE_HOME/look/cache` or `~/.cache/look/cache`

Set `LOOK_CACHE_DIR` to override the complete cache path.

The session server writes `server.json` in the same directory. That state file
contains its loopback address, process ID, and authentication token. Live scene
data and GPU resources remain in memory and disappear when the server stops.

Removing the cache directory while the server is stopped is safe; metadata will
be regenerated on the next inspection. Use `look server stop --json` before
removing it while a session is active.
