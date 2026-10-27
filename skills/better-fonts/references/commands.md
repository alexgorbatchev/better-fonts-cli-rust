---
created_on: 2026-10-14 11:57
last_modified: 2026-10-14 11:57
status: current
---

## Commands

- `better-fonts`: show help when no command is supplied.
- `better-fonts app`: application command group.
- `better-fonts app patch [APP]...`: patch selected applications. APP accepts IDs, case-insensitive names, exact registered paths, existing `.app` paths, `*`, or `all`. Omitted targets use configured apps (default all).
- `better-fonts app unpatch [APP]...`: restore selected applications, using the same APP selection. A clean app needs no mutation.
- `better-fonts app status [APP]...`: inspect installed/patched/font status; omitted APP inspects every registered app.
- `better-fonts app list`: list built-in and custom app definitions.
- `better-fonts config`: configuration command group.
- `better-fonts config path`: print the effective path without creating a file.
- `better-fonts config init`: create annotated defaults if absent, preserving existing configuration.
- `better-fonts config show`: create defaults if absent and print effective settings as TOML.
- `better-fonts self`: executable command group.
- `better-fonts self upgrade`: update the running executable from an explicitly configured Rust release channel.
- `better-fonts completion`: completion command group.
- `better-fonts completion script <shell>`: print a native completion script. shell is bash, zsh, fish, or powershell; output is a script in both modes.
- `better-fonts skill`: print the Skills CLI installation command and canonical skill URL, with identical bytes in both modes. Accepts global options and no additional positional arguments or command-specific flags; performs no network or configuration access.
- `better-fonts help [COMMAND...]`: show help for the named command path. `--help` and `-h` show help on every command. Agent help starts with the skill alert.

## Global flags

| Flag | Type | Default | Contract |
| :--- | :--- | :--- | :--- |
| `--config`, `-c` | PathBuf | unset; resolved XDG config path | TOML file path |
| `--verbose`, `-v` | bool | false | Include detailed human per-app mutation results |
| `--version` | bool | false | Print raw 0.1.0 and newline, without labels |
| `--help`, `-h` | bool | false | Print help without executing a mutation |

## Mutation flags

| Flag | Type | Default | Contract |
| :--- | :--- | :--- | :--- |
| `--font`, `-f` | String | unset; per-app then global font | patch only; nonempty name without NUL |
| `--driver` | enum value | unset; registered or autodetected | electron or native-hook |
| `--restart` | bool | unset; per-app then global true | Optional value requires `=`; bare flag means true |
| `--no-restart` | bool | false | Set restart false; conflicts with --restart |
| `--dry-run` | bool | false | Inspect targets without compiling, signing, quitting, or replacing bundles |

Completion flag `--no-descriptions` has type bool, default false; it removes descriptions from generated scripts.

Read [configuration](configuration.md) for all environment variables and [operations](operations.md) for outputs and side effects.
