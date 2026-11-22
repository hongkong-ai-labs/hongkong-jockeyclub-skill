# hkjc

Hong Kong Jockey Club horse racing, football and Mark Six as a Rust CLI and MCP
server.

Reads the official, keyless data that powers bet.hkjc.com: race meetings, race
cards, runners, live tote odds, football matches and odds, Mark Six draws and
prize divisions. Historical race results, dividends, season fixtures and
jockey, trainer and horse profiles are read from racing.hkjc.com.

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
hkjc card --race 7
hkjc odds --race 1 --types WIN,PLA,QIN
hkjc results --date 2026-11-18 --venue HV --race 4
hkjc fixtures
hkjc fixtures --all
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

Add `--json` for structured output.

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
`hk_race_fixtures`, `hk_race_profile`, `hk_race_lookup`, `hk_football_matches`,
`hk_football_match`, `hk_marksix_results`, `hk_marksix_latest`,
`hk_marksix_stats`.

## Data

- `https://info.cld.hkjc.com/graphql/base/` — race meetings, race cards,
  runners, tote odds, Mark Six.
- `https://racing.hkjc.com/racing/information/English/Racing/LocalResults.aspx`
  — historical race results and dividends.
- `https://racing.hkjc.com/racing/information/English/Racing/Fixture.aspx`
  — the season fixture calendar.
- `https://racing.hkjc.com/en-us/local/information/` — jockey, trainer and horse
  profiles.

`meetings` covers only the meeting HKJC currently publishes; `fixtures` covers
the season, including future meetings. Historical results need an actual race
day. Mark Six history is a capped recent window, and an explicit date range must
span about three months or less. Odds, declarations and results change; check
HKJC for confirmation.

## Run modes and deployment

| Mode | Start with | Use for |
| --- | --- | --- |
| CLI | `hkjc <command>` | Interactive use and shell scripts |
| JSON | `<command>` with `--json` | `jq`, automation, other programs |
| MCP stdio | `hkjc mcp` | Desktop and CLI MCP clients |
| Container | `docker run --rm -i hkjc mcp` | No local Rust toolchain |

### MCP and authentication

This server is **stdio-only**: the client launches the process and speaks MCP
over stdin/stdout. There is no listening port, so there is no network
authentication layer and no API-key setting. Optional API keys in this project
family apply only to servers that expose an HTTP transport. Keep the process
local and let the MCP client manage it.

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
HTTP APIs over TLS and uses a Tokio runtime; packaging it as a stdio server is
the supported integration. A Worker would need a separate HTTP/JavaScript
adapter and is out of scope for this repo.

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
