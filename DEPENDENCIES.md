---
created_on: 2026-10-14 11:57
last_modified: 2026-10-14 11:57
status: current
---

# Native dependencies and data contracts

The source is [alexgorbatchev/better-fonts-cli](https://github.com/alexgorbatchev/better-fonts-cli). MIT attribution applies to the derived CoreText interposition logic and app definitions. `Cargo.lock` fixes the independently built Rust dependency graph.

| Source responsibility | Rust implementation |
| :--- | :--- |
| Cobra command parsing | [Clap](https://docs.rs/clap/latest/clap/), with direct cli-command-kit help and skill contracts |
| Generated shell scripts | [clap_complete](https://docs.rs/clap_complete/latest/clap_complete/) native Generator::try_generate, preserving write failures |
| TOML settings | [toml](https://docs.rs/toml/latest/toml/) and Serde; atomic tempfile persistence |
| XML/binary bundle plist | [plist](https://docs.rs/plist/latest/plist/) Value parsing |
| OS paths and attributes | dirs home resolution, native std filesystem/process APIs, [xattr](https://docs.rs/xattr/latest/xattr/) without symlink dereferencing |
| Artifact hashing and integrity | SHA256 via sha2; native std file locks; tempfile adjacent stages |
| Upgrade release/checksum/install | Native cli-deps Updater, explicitly configured Rust release channel |
| Apple font hook and executable launcher | Embedded Objective-C/C assets compiled by Apple clang against CoreText/Foundation, preserving Mach-O DYLD_INTERPOSE semantics |

## Archive and fuse implementation

The checked ASAR replacement follows the [Electron archive disk format](https://github.com/electron/asar/blob/main/src/disk.ts) and [file integrity format](https://github.com/electron/asar/blob/main/src/integrity.ts). Its serde_json header retains unknown fields; only packed offsets, the selected preload's size, and its existing SHA256 integrity change. Other entries retain byte content and metadata.

Inspected alternatives do not meet this lossless edit contract: asar 0.3.0 omits executable metadata in reader-to-writer copying and cannot preserve the complete header; hive-asar 0.4.0 documents unpacked/integrity gaps; asar-rust 0.1.0 has lossy header types and panic-based malformed Pickle reads. A checked format edit is therefore necessary. It rejects invalid headers, unsafe entry names, unsupported integrity algorithms, and target entries that are unpacked or links.

Fuse detection follows [Electron fuses](https://github.com/electron/fuses/blob/main/src/index.ts), including overlap across stream chunks, universal binary occurrences, version/state checks, and validation of all wires before writing v1 integrity index 4.

## Correctness and operational differences

- Explicit font and restart flags take precedence over per-app/global settings. Omitted restart preserves configured false, rather than overwriting it with the parser's default true.
- Font values are encoded as JavaScript JSON/CSS strings or C octal UTF-8 bytes. Patch removal parses bounded injection blocks and preserves neighboring code/newlines. Existing persisted Go blocks remain readable.
- Quit/sign/compile failures propagate before bundle replacement. A failed publication restores the original when possible; failed restoration preserves and identifies the backup. Post-publication restart failures retain a committed result with exit 6.
- Adjacent persistent file locks coordinate the same canonical bundle across different cache settings. Temporary stages/build directories have unique names; subprocess cancellation terminates process groups.
- XDG environment overrides remain supported. Fallbacks use the source project's documented `~/.config` and `~/.cache` paths; the Go implementation's platform UserConfigDir/UserCacheDir calls otherwise select Application Support/Caches on macOS, contrary to that documentation.
- Native launchers execute the preserved `.orig` even when the executable name contains Launcher, retain caller DYLD libraries, and use dynamically sized paths. CoreText descriptor overrides retain other descriptor attributes. Hook diagnostics write to stderr.
- Custom ASAR paths must remain inside the bundle. Unknown explicit app targets fail, rather than reporting successful work without a selected app.
- The public domain command tree uses `app patch/unpatch/status/list`, `config path/show/init`, and `self upgrade`; it retains native shell completion generation under `completion script`.

Actual macOS CoreText rendering, application launch, ad-hoc signature acceptance, hardened-runtime restrictions, and Gatekeeper behavior require a Mac. Linux tests verify filesystem/archive/argument/recovery orchestration and native cancellation, without proving those Apple runtime outcomes.
