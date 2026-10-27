`better-fonts` applies a chosen font to macOS desktop applications. It supports Electron preload patches and native CoreText hooks, with per-app settings and recovery copies.

# What It Does

- **Choose fonts:** Apply a global font, a per-app override, or an explicit command option.
- **Inspect apps:** List supported applications and report installed patches.
- **Restore fonts:** Remove injected Electron code or restore a native executable.
- **Preserve bundles:** Prepare and sign a separate copy before replacing an app.

# How It Works

- Inspect applications with `better-fonts app status`.
- Select app IDs, app names, or an application bundle path when patching.
- Use `--dry-run` to inspect targets without compiling or changing bundles.
- Apply the selected font and optionally restart each application.

# How it Really Works

- Settings live in `$XDG_CONFIG_HOME/better-fonts/config.toml`, falling back to `~/.config/better-fonts/config.toml`. Build artifacts and process captures use `$XDG_CACHE_HOME/better-fonts`, falling back to `~/.cache/better-fonts`.
- Electron patches edit the configured packed preload, preserve other archive contents and metadata, disable embedded archive integrity validation, and ad-hoc sign the staged app. Unpatch removes the font injection; the integrity fuse remains disabled.
- Native patches compile a font-specific CoreText hook and matching launcher. The original executable is preserved as `.bak` and `.orig`; unpatch restores it and retains `.bak`.
- Bundle replacement requires write permission beside the application and enough space for staged and original copies. A persistent adjacent lock coordinates writers. Compilation, signing, and quit failures leave the original bundle intact; publication failure attempts restoration.
- Restart failure after replacement exits 6 because the patch is already committed. Cancellation exits 130; invalid arguments exit 2; other failures exit 1. Success exits 0.
- Human mode prints readable lines; `AGENT=1`, `true`, or `yes` prints app results as newline-delimited JSON. Returned errors appear once on stderr. Displayed home paths fold to `~`.
- Archive and recovery tests run locally on Linux. CoreText rendering, macOS launch, and signing acceptance require a macOS runtime check.

# Prerequisites

- macOS and an installed target application for bundle mutation.
- [Apple Command Line Tools](https://developer.apple.com/xcode/resources/) providing `clang` and `lipo` for native hooks, plus macOS `codesign`; restarting uses `osascript` and `open`.
- The requested font installed on the Mac.

# Installation

Rust artifacts are unpublished. Existing Go release archives are separate artifacts and do not install this Rust implementation. The local executable reports version `0.1.0`.

# Quick Start

```sh
better-fonts --version
better-fonts config init
better-fonts app status
better-fonts app patch slack --font 'JetBrains Mono' --dry-run
better-fonts app patch slack --font 'JetBrains Mono' --no-restart
better-fonts app unpatch slack --no-restart
```

Sample Output for `better-fonts --version`:

```text
0.1.0
```

# Options & Flags

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--config <path>` | `-c` | XDG config path above | Read this TOML file |
| `--verbose` | `-v` | `false` | Include font, restart setting, and path in human mutation results |
| `--version` | | `false` | Print only the version and newline |
| `--help` | `-h` | | Show command help |

`better-fonts app patch` and `better-fonts app unpatch`:

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--font <name>` | `-f` | Per-app font, then global `Maple Mono Normal NF` | Patch only: override the font |
| `--driver <driver>` | | App registry or automatic detection | Choose `electron` or `native-hook` |
| `--restart[=true\|false]` | | Per-app setting, then global `true` | Explicitly override restart behavior |
| `--no-restart` | | `false` | Override restart behavior to false |
| `--dry-run` | | `false` | Inspect without compilation or bundle mutation |

`better-fonts completion script <shell>`:

| Flag | Short | Default | Description |
| :--- | :--- | :--- | :--- |
| `--no-descriptions` | | `false` | Omit descriptions from bash, zsh, fish, or powershell completion |

# Configuration

```toml
font = 'Maple Mono Normal NF'
apps = ['slack', 'telegram']
restart = false

[apps_config.slack]
font = 'JetBrains Mono'
restart = true
```

Explicit font and restart options take precedence over per-app and global settings. See the [operational skill](skills/better-fonts/SKILL.md) for custom app definitions, output fields, and all commands. The offline `skill` command prints the canonical source skill location; publication of the Rust repository and skill is pending.

# License

[MIT](LICENSE), Alex Gorbatchev.
