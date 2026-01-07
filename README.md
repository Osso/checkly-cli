# checkly

Rust CLI for querying Checkly synthetic monitoring check failures.

## Installation

```bash
cargo build --release
ln -s $(pwd)/target/release/checkly ~/bin/checkly
```

## Configuration

```bash
checkly config --api-key <key> --account-id <id>
```

Get credentials from Checkly dashboard:
- API Key: Account Settings → API Keys
- Account ID: Account Settings (shown in URL)

Config stored in `~/.config/checkly-cli/config.json`.

## Usage

```bash
# List all checks
checkly checks

# Show current status of all checks
checkly status

# Only show failing checks
checkly status --failures-only

# Show failures for a specific check (last 6 hours)
checkly failures <check-id>

# Show failures with custom time range
checkly failures <check-id> --since 24h
checkly failures <check-id> --since 7d

# JSON output (all commands)
checkly status --json
checkly failures <check-id> --json
```

## Time Range Format

- `m` - minutes (e.g., `30m`)
- `h` - hours (e.g., `6h`, `24h`)
- `d` - days (e.g., `7d`)

## API Rate Limits

- Check results: 5 requests / 10 seconds
- Results retention: 30 days (raw), indefinite (aggregated)
- Max time range per request: 6 hours (CLI handles chunking automatically)
