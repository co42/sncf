# sncf

SNCF train schedule CLI.

## Install

From source:

```bash
cargo install --git https://github.com/co42/sncf
```

## Auth

Get a free API key from [SNCF Open Data](https://numerique.sncf.com/startup/api/) and export it:

```bash
export SNCF_API_KEY="your-key-here"
```

## Commands

### Search stations

```bash
sncf search "Lyon Part-Dieu"
sncf search "Paris" --limit 5
```

### Next trains

```bash
sncf next "Lyon Part-Dieu" "Paris Gare de Lyon"
sncf next "Lyon Part-Dieu" "Paris Gare de Lyon" --limit 10
sncf next "Lyon Part-Dieu" "Paris Gare de Lyon" --at 07:00
sncf next "Lyon Part-Dieu" "Paris Gare de Lyon" --date 2026-03-15
```

Station names are resolved automatically. You can also use stop_area IDs directly:

```bash
sncf next "stop_area:SNCF:87723197" "stop_area:SNCF:87686006"
```

### Disruptions

```bash
sncf disruptions                          # All current disruptions
sncf disruptions --station "Lyon Part-Dieu"
sncf disruptions --line "TGV INOUI"
```

### Shell completions

```bash
sncf completions bash >> ~/.bashrc
sncf completions zsh >> ~/.zshrc
sncf completions fish > ~/.config/fish/completions/sncf.fish
```

## Station aliases

Create `~/.config/sncf/aliases.toml` to define short names for stations:

```toml
home = "Lyon Part-Dieu"
work = "Paris Gare de Lyon"
```

Then use them as station names:

```bash
sncf next home work
```

## Output

Output format is auto-detected from TTY. Override with:

```bash
sncf next "Lyon" "Paris" --json          # Force JSON
sncf next "Lyon" "Paris" --no-json       # Force human-readable
sncf next "Lyon" "Paris" --json --compact # Compact JSON (no pretty-printing)
sncf next "Lyon" "Paris" --json --fields departure,arrival,train_type
sncf next "Lyon" "Paris" -q              # Quiet (suppress output)
```

JSON output uses ISO 8601 timestamps and omits null fields. Errors in JSON mode include a structured `code` field for programmatic handling.
