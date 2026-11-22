# hkjc

Hong Kong Jockey Club horse racing, football and Mark Six as a Rust CLI and MCP
server.

Reads the official, keyless data that powers bet.hkjc.com: race meetings, race
cards, runners, live tote odds, football matches and odds, Mark Six draws and
prize divisions. Historical race results, dividends, season fixtures,
championship standings, barrier trials, sectional times and jockey, trainer and
horse profiles are read from racing.hkjc.com.

## Install

**Agent skill** — installs `SKILL.md` and the binary bootstrap script:

```bash
npx skills add hongkong-ai-labs/hongkong-jockeyclub-skill
npx skills update        # refresh the installed skill later
```

**Prebuilt binary** — macOS (Intel or Apple silicon) and Linux (x86_64 or
aarch64, static musl), checksum-verified, into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/hongkong-ai-labs/hongkong-jockeyclub-skill/main/scripts/install.sh | sh
```

**From source** — Rust 1.89 or newer:

```bash
git clone https://github.com/hongkong-ai-labs/hongkong-jockeyclub-skill
cd hongkong-jockeyclub-skill
cargo install --path . --locked
# or directly from GitHub:
cargo install --git https://github.com/hongkong-ai-labs/hongkong-jockeyclub-skill --locked
# or: cargo build --release --locked   # target/release/hkjc
```

Verify:

```console
$ hkjc --version
```

`hkjc update` detects whether the binary came from cargo, the release
installer, a source checkout or Homebrew, and updates with that same
method. `hkjc update --check` only reports.

## CLI

```bash
hkjc meetings
hkjc meetings --venue ST
hkjc --lang zh meetings              # Traditional Chinese labels and names
hkjc card --race 7
hkjc odds --race 1 --types WIN,PLA,QIN
hkjc results --date 2026-11-18 --venue HV --race 4
hkjc standings jockey --season 26/27
hkjc trials --date 2026-11-18
hkjc sectionals --date 2026-11-18 --race 4
hkjc pace --date 2026-11-18 --race 4
hkjc odds --race 1 --record          # save an odds snapshot for history
hkjc history --date 2026-11-18 --venue HV --race 4
hkjc watch --race 1 --interval 60    # poll odds and scratchings
hkjc marksix-check 1,2,3,4,5,6
hkjc doctor
hkjc fixtures
hkjc fixtures --all
hkjc fixtures --next
hkjc fixtures --ics > races.ics
hkjc completions zsh > _hkjc      # bash, zsh, fish, elvish, powershell
hkjc man > hkjc.1                 # roff manual page
hkjc lookup Purton
hkjc profile jockey PZ
hkjc profile horse HK_2024_K392
hkjc football --limit 20
hkjc football-match 50079900
hkjc marksix --draws 10
hkjc marksix --from 2026-11-01 --to 2026-11-21
hkjc marksix-latest
hkjc marksix-stats --draws 100
hkjc mcp
hkjc update --check
```

Add `--json` for structured output, or `--csv` for RFC 4180 rows that open
directly in a spreadsheet. `--csv` and `--json` are mutually exclusive.
`--lang zh` (also `zh-Hant`, `zh-HK`) switches the human-readable tables and
messages to Traditional Chinese, using Chinese horse, jockey, trainer, team and
venue names when HKJC publishes them. JSON and CSV keep stable English field
names and values.

`watch` polls the live card and reports odds moves and scratchings. Add
`--record` to store each poll, `--webhook URL` to POST changes as JSON, or
`--telegram BOT_TOKEN:CHAT_ID` to send them to Telegram. `--count` stops
after a fixed number of polls. Live responses are cached briefly, so polls
run at five-second intervals or slower.

Odds history is stored in SQLite at `$HKJC_DB`, else
`$XDG_DATA_HOME/hkjc/history.sqlite`, else
`~/.local/share/hkjc/history.sqlite`. Pass `--db PATH` to use another file.

## MCP

```json
{
  "mcpServers": {
    "hongkong-jockeyclub": { "command": "hkjc", "args": ["mcp"] }
  }
}
```

Register it with any MCP client:

```bash
npx add-mcp "$(command -v hkjc) mcp" --name hongkong-jockeyclub -a claude-code -a codex -a cursor -y
claude mcp add hongkong-jockeyclub -- hkjc mcp
codex mcp add hongkong-jockeyclub -- hkjc mcp
```

`add-mcp` detects the client configuration files in the current project; add
`-g` for a global install. `$(command -v hkjc)` keeps the entry correct
when the binary is on `PATH`; substitute an absolute path if your client does
not inherit the shell `PATH`.


Tools: `hk_race_meetings`, `hk_race_card`, `hk_race_odds`, `hk_race_results`,
`hk_race_fixtures`, `hk_race_standings`, `hk_race_trials`, `hk_race_sectionals`,
`hk_odds_history`, `hk_race_profile`, `hk_race_lookup`, `hk_football_matches`,
`hk_football_match`, `hk_marksix_results`, `hk_marksix_latest`,
`hk_marksix_stats`, `hk_marksix_check`, `hk_source_health`.

### HTTP transport

For clients that cannot launch a local process, serve the same tools over
streamable HTTP:

```bash
hkjc mcp --http                       # http://127.0.0.1:8765/mcp
hkjc mcp --http 0.0.0.0:8765 --token "$HKJC_MCP_TOKEN"
```

`POST /mcp` takes one JSON-RPC request and returns one JSON response;
notifications return `202 Accepted`. `GET` is not supported because this server
has no server-initiated stream. The endpoint is stateless: there is no session
id to carry between requests.

Binding a non-loopback address requires a bearer token, which can be supplied
with `--token` or `HKJC_MCP_TOKEN`. Requests must send
`Authorization: Bearer <token>`. The server also rejects browser `Origin`
headers that are not localhost. Keep the listener behind a trusted network or
reverse proxy; the token is the only authentication layer.

## Data

- `https://info.cld.hkjc.com/graphql/base/` — race meetings, race cards,
  runners, tote odds, Mark Six.
- `https://racing.hkjc.com/racing/information/English/Racing/LocalResults.aspx`
  — historical race results and dividends.
- `https://racing.hkjc.com/racing/information/English/Racing/Fixture.aspx`
  — the season fixture calendar.
- `https://racing.hkjc.com/en-us/local/information/` — jockey, trainer and horse
  profiles, jockey and trainer rankings, barrier trials and sectional times.

`meetings` covers only the meeting HKJC currently publishes; `fixtures` covers
the season, including future meetings. Historical results, barrier trials and
sectional times need an actual race or trial day. Mark Six history is a capped
recent window, and an explicit date range must span about three months or less.
HKJC's public football endpoints publish matches and odds, not league tables, so
`hkjc` does not fabricate standings.

Standings default to the current season; pass `--season 25/26` for a previous
one. Odds, declarations and results change; check HKJC for confirmation.

## Run modes and deployment

| Mode | Start with | Use for |
| --- | --- | --- |
| CLI | `hkjc <command>` | Interactive use and shell scripts |
| JSON | `<command>` with `--json` | `jq`, automation, other programs |
| CSV | `<command>` with `--csv` | Spreadsheets and data pipelines |
| Watch | `hkjc watch` | Odds moves, scratchings and webhook alerts |
| MCP stdio | `hkjc mcp` | Desktop and CLI MCP clients |
| MCP HTTP | `hkjc mcp --http` | Remote MCP clients and containers |
| Container | `docker run --rm -i hkjc mcp` | No local Rust toolchain |

### MCP and authentication

The default transport is **stdio**: the client launches the process and speaks
MCP over stdin/stdout, with no listening port. The optional HTTP transport is
started explicitly with `hkjc mcp --http`; it requires a bearer token when it
listens beyond loopback. Keep the stdio process local and let the MCP client
manage it.

Typical client entry:

```json
{
  "mcpServers": {
    "hongkong-jockeyclub-skill": {
      "command": "hkjc",
      "args": ["mcp"]
    }
  }
}
```

If the client does not inherit `PATH`, use the absolute installed path.

### Docker and Apple container

```bash
docker build -t hkjc .
docker run --rm hkjc --help
docker run --rm -i hkjc mcp          # stdio MCP
docker run --rm -p 8765:8765 -e HKJC_MCP_TOKEN="$TOKEN" \
  hkjc mcp --http 0.0.0.0:8765       # remote MCP
docker compose run --rm stdio             # same, through Compose
```

Apple's `container` runtime uses the same OCI image:

```bash
container build -t hkjc .
container run --rm hkjc --help
container run --rm -i hkjc mcp
```

The runtime image contains the release binary and CA certificates, runs as a
non-root user, and defaults to `--help` so a bare `docker run` is harmless.

### Cloudflare and WASM

There is no Worker or WASM deployment. The binary talks directly to upstream
HTTP APIs over TLS and uses a Tokio runtime; stdio and the built-in HTTP
transport are the supported integrations. A Worker would need a separate
HTTP/JavaScript adapter and is out of scope for this repo.

### Releases

Pushing a `v*` tag runs the release workflow, which checks the tag against
`Cargo.toml` and `CHANGELOG.md`, builds four targets, smoke-tests native
binaries and publishes:

- `hkjc-<tag>-<target>.tar.gz` for macOS and static Linux, containing both
  `hkjc` and `hkjc-mcp`
- `SHA256SUMS` covering every tarball

`scripts/install.sh` downloads the latest release, verifies its checksum and
falls back to `cargo build --release --locked` when no release is available.
`just tag X.Y.Z` creates the annotated tag locally; `just release X.Y.Z` tags
and pushes to trigger the workflow.

## Development

```bash
just                 # fmt check, clippy with -D warnings, tests
just skill-check     # validate SKILL.md and shell syntax
just container       # build the image and print CLI help
cargo test --all --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

`rust-toolchain.toml` pins the stable toolchain with `rustfmt` and
`clippy`, so `rustup` sets contributors up automatically. `just doc`
builds the API documentation. See `CONTRIBUTING.md` for the full check
list and the signed-commit requirement.


CI runs the same checks. Dependabot proposes weekly Cargo and GitHub Actions
updates, and the audit workflow checks the lockfile against the RustSec
advisory database.

## Licence

MIT. The data remains subject to the Hong Kong Jockey Club's terms of use.
