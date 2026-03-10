# sncf

CLI tool for querying SNCF train schedules via the Navitia API.

## Structure

- `src/main.rs` — clap CLI, command dispatch
- `src/lib.rs` — module exports
- `src/client.rs` — SNCF/Navitia API client
- `src/error.rs` — thiserror error types
- `src/output.rs` — JSON/human output (TTY auto-detect)
- `src/commands/search.rs` — station search
- `src/commands/next.rs` — next trains between stations

## Auth

`SNCF_API_KEY` env var. Free key from SNCF open data.

## Usage

```
sncf search <query> [--limit N]
sncf next <from> <to> [--limit N] [--at HH:MM]
```

Global flags: `--json`, `--no-json`, `--fields f1,f2`, `-q`/`--quiet`
