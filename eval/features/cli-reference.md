# `ctsync` command reference

`ctsync` is the command-line companion of Contoso Sync.

## Synopsis

```
ctsync <command> [--profile <name>] [--verbose]
```

## Commands

| Command | Description |
|---|---|
| `ctsync status` | Shows the sync state of every folder |
| `ctsync pause --minutes 30` | Pauses sync for the given time |
| `ctsync resume` | Resumes a paused sync |
| `ctsync reset --force` | Deletes the local cache and downloads everything again |
| `ctsync logs --tail` | Streams the log file `ctsync.log` |

## Global options

- `--profile <name>`: Uses the named profile from `~/.ctsync/config.toml`.
- `--verbose`: Prints debug output. Combine it with `--no-color` when you redirect output to a file.
- `--json`: Prints machine-readable output.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General error |
| 3 | Not signed in |
| 7 | The folder is locked by another process |

## Examples

Pause sync during a presentation:

```bash
ctsync pause --minutes 60
```

Check the state of a single folder and save it as JSON:

```bash
ctsync status --json > status.json
```
