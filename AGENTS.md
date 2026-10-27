---
created_on: 2026-10-14 11:57
last_modified: 2026-10-14 11:57
status: current
---

# better-fonts

## Commands

- Build: `just build` writes `bin/better-fonts`.
- Run: `just run app status`; agent mode: `just run-ai app status`.
- Format: `just fmt`; strict lint: `just lint`; tests: `just test`.
- Full verification: `just check` includes at least 90% line coverage.
- Development requires Rust 1.89+, just, cargo-llvm-cov and the LLVM tools component.

## Contracts

- Consult applicable skills before edits. Update `skills/better-fonts/SKILL.md` and its references in the same change as command, argument, option, default, environment, output, or side-effect changes. Verify with the native command tree and update document metadata.
- Any runtime-result change requires a corresponding test-file change and 90% line coverage; `scripts/` is excluded. Record red/green and genuinely disabled implementation failures followed by exact restoration.
- Scope formatting to this package: `cargo fmt --package better-fonts-cli`. Path dependencies are independent repositories.
- Preserve all unknown ASAR JSON metadata, unpacked entries, links, empty directories, untouched file integrity, executable flags, permissions, and extended attributes. Recalculate the edited file's existing SHA256 integrity using its recorded block size.
- Validate every Electron fuse occurrence before modifying any occurrence. Preserve universal binaries; integrity fuse v1 index 4 remains disabled on unpatch, matching the actual persisted archive operation.
- Keep compiled native assets architecture matched to the application. The Objective-C CoreText hook and C Mach-O launcher require Apple SDK tooling and actual macOS acceptance checks. Linux orchestration tests do not establish CoreText or Gatekeeper acceptance.
- Preserve CFBundleExecutable XML/binary plist behavior, `.bak` recovery, `.orig` launch targets, existing DYLD insertion libraries, private/system/icon font exclusions, and descriptor attributes.
- Stage a complete adjacent bundle, preserve symlinks and extended attributes, compile and sign before publication, and restore the original on publication failure. Keep the adjacent app lock inode persistent so concurrent cache settings share one lock.
- Propagate cancellation through native process groups. Report restart failures after publication with the committed error class and exit 6.
- Keep raw version output and skill-pointer bytes stable; print returned errors once in main. Route all displayed paths through shared home folding. Keep absolute/native paths for filesystem and process operations.
- Rust artifacts are unpublished. A configured self-updater uses the three explicit Rust release variables; never substitute the Go release channel.
- Never publish releases, tags, packages, or production deployments without explicit user authorization. Keep binaries, temporary files, and evidence ignored. Preserve MIT attribution.

## Verification boundaries

Use synthetic archives and isolated app bundles under project `.tmp/`, with real filesystem locks and subprocess cancellation tests. Tests inject Apple tool invocations without modifying installed apps. A real macOS patch requires separate verification of rendering, architecture, launch, signing, restart, and unpatch behavior.
