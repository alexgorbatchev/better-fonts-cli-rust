---
created_on: 2026-10-14 11:57
last_modified: 2026-10-14 11:57
status: current
---

## Bundle mutation

Use macOS with the selected font installed and writable target bundle parent. Native hooks need Apple clang/lipo and CoreText/Foundation SDKs. Both drivers use codesign for ad-hoc signing; restart invokes osascript to quit, then open -a after replacement.

Electron edits the configured packed preload, preserving other file bytes, unknown archive metadata, unpacked entries, links, empty directories, executable flags, permissions, and extended attributes. It recalculates the edited file's existing SHA256 integrity. Font CSS excludes icon elements and preserves Anthropic icon fonts. Existing persisted Go injection blocks are recognized for removal/replacement. Unpatch retains disabled Electron embedded-ASAR validation.

Native hooks select CFBundleExecutable from XML/binary plist or require one unambiguous executable. They preserve original executable .bak and .orig files, compile architecture-matched hook/launcher artifacts, and store the configured font beside them. The launcher executes .orig and preserves other DYLD inserted libraries. Native unpatch restores .bak, falling back to .orig, removes hook/launcher metadata, and retains .bak. Existing native patches without metadata report an unknown font as null.

Compilation cache keys include font, architectures, and hook/launcher sources. Complete cached artifacts avoid compilation. App bundle replacements hold a persistent adjacent lock, copy the whole bundle into an adjacent stage, sign before quitting, and publish by renaming. Failure before publication preserves the original. Publication failure attempts restoration; a failed restoration reports the retained original's path. Restart or backup cleanup failure after replacement returns exit 6. Separate app selections may already be committed when another app fails.

Dry-run resolves and inspects app status, with no compilation or bundle mutation; it may initialize absent configuration. Missing registered apps produce not-installed records; unknown explicit app selections fail. Cancellation kills in-flight subprocess groups and stops subsequent work. A cancellation after replacement can still accompany a committed error.

## Outputs

Agent app list records contain `app`, `id`, `driver`, `path`. Status adds `installed`, `patched`, `font` (string or null). Successful mutation records contain `app`, `state` (patched/unpatched/dry-run), `path`, `font`, `restart`. Missing installed targets emit `app`, `state` (not-installed), `path`. Records are newline-delimited JSON on stdout; paths in these display records use `~` under home.

Human app commands print one readable line per app. Config path/init print a path; config show prints TOML. Version prints its raw value; completion prints a shell script; skill prints the exact offline pointer. Failures return through main once on stderr. Runtime errors have ERR prefix; parse errors use native parser diagnostics. self upgrade prints the native updater outcome after success.

Use exit 0 for success, 1 for filesystem/native tool/configuration/release failures, 2 for invalid command arguments, 6 for an already committed mutation with subsequent failure, and 130 for cancellation.

CoreText rendering and macOS launch/signature acceptance are runtime verification boundaries; Linux fixture tests cover archive and orchestration behavior.
