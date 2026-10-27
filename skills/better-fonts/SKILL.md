---
name: better-fonts
description: Use when operating better-fonts to inspect, patch, or restore macOS application fonts and manage its settings.
author: alexgorbatchev
metadata:
  created_on: 2026-10-14 11:57
  last_modified: 2026-10-14 11:57
  status: current
---

Read [commands](references/commands.md) for every command and option, [configuration](references/configuration.md) for defaults and custom apps, and [operations](references/operations.md) before changing an app bundle.

Read [generated help queries](references/help.md) when requesting help through a command path.

Use `AGENT=1` for compact app records. Start with `better-fonts app status`, then preview a specific target:

```sh
AGENT=1 better-fonts app patch slack --font 'JetBrains Mono' --dry-run
AGENT=1 better-fonts app patch slack --font 'JetBrains Mono' --no-restart
AGENT=1 better-fonts app unpatch slack --no-restart
```

Treat exit 6 as a committed replacement followed by a restart or cleanup failure. Inspect the installed app before retrying. Exit 130 means cancellation; 2 means argument parsing failed; 1 means another failure; 0 means success.

Use `better-fonts skill` for the offline canonical skill pointer. The local Rust implementation is version 0.1.0 with unpublished artifacts; the pointer identifies the source repository, whose Go releases are distinct artifacts.
