---
created_on: 2026-10-14 11:57
last_modified: 2026-10-14 11:57
status: current
---

Resolve configuration as `$XDG_CONFIG_HOME/better-fonts/config.toml`, otherwise `~/.config/better-fonts/config.toml`. Resolve build/process cache as `$XDG_CACHE_HOME/better-fonts`, otherwise `~/.cache/better-fonts`. Empty XDG variables select the fallback. `HOME` supplies the platform home resolution. CLI `--config` overrides the configuration path.

Set `AGENT=1`, `true`, or `yes` for app NDJSON records; unset, 0, or other values select human output. Printed home paths use `~`; actual files retain native paths.

Global TOML fields:

- `font`: string, default `Maple Mono Normal NF`; empty font selects that default.
- `apps`: array of target strings, default `['*']`; an empty array selects all.
- `restart`: boolean, default true.
- `apps_config`: table keyed by app ID, each with `font` string and optional `restart` boolean. A blank per-app font inherits global.
- `custom_apps`: array of app tables, default empty.

Explicit `--font` and restart options override per-app settings, which override global settings. App selection accepts trimmed case-insensitive IDs/names and wildcards. Duplicate app paths execute once.

Custom app fields: `id` (required unique ASCII letters/digits/hyphen/underscore), `name` (string), `path` (required bundle path), `driver` (electron default or native-hook), `process_name` (application name for quit/open; empty skips restart commands), `asar_path` (default app.asar relative to Contents/Resources), and `preload_path` (required for Electron, packed archive path). `unpack_flags` is retained configuration data with no runtime effect because archive rewrites preserve unpacked entries directly.

Built-in IDs: paseo, signal, slack, rekordbox, engine-dj, telegram, chatgpt, claude. Locations default to Applications; ChatGPT and Claude also check the home Applications directory. Rekordbox checks version 7, version 6, then the flat bundle location. Slack chooses its architecture-specific ASAR when present. Arbitrary Electron bundles require exactly one ASAR and default preload.js; configure custom_apps for other layouts.

Native runtime `BETTER_FONTS_NAME` overrides the compiled hook font when valid UTF-8. `BETTER_FONTS_DEBUG=1` enables hook diagnostics on stderr.

For `self upgrade`, configure all three nonempty values: `BETTER_FONTS_RUST_RELEASE_BASE_URL`, `BETTER_FONTS_RUST_RELEASE_OWNER`, `BETTER_FONTS_RUST_RELEASE_REPO`. The base URL uses HTTPS except loopback HTTP for explicit local testing. Omitted channel returns a typed unpublished error; partial channel is invalid. The updater verifies release checksums and replaces the running executable; source Go release URLs are separate from this channel.
