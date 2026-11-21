---
name: hongkong-jockeyclub-skill
description: Hong Kong Jockey Club data — horse racing fixtures, race cards, runners, jockeys, trainers, horse profiles, barriers, live tote odds, historical results, dividends and incidents, HKJC football matches and odds, plus Mark Six draw results, jackpots, prize divisions and number frequency. Use for questions about Hong Kong racing, Sha Tin, Happy Valley, Mark Six, 六合彩, race results, football or lottery numbers.
license: MIT
---

# Hong Kong Jockey Club

`hkjc` is a Rust CLI and MCP server for Hong Kong Jockey Club horse racing,
football and Mark Six. It needs no API key and no account. It is an independent,
unofficial project.

Use it when the user asks about:

- today's or the current Hong Kong race meeting, races and post times
- a race card: runners, horse names, jockeys, trainers, barriers, weights,
  ratings and recent form
- live tote odds for a race (WIN, PLA, QIN, QPL, FCT, TCE, TRI, FF, QTT, CWA…)
- a past race result: finishing order, lengths behind, times, starting odds,
  dividends and stewards' incidents
- the season fixture list, including future race meetings weeks ahead
- a jockey, trainer or horse profile: age, background, season statistics and
  recent form
- HKJC football: match list, status, scores, HAD odds and full odds boards
- Mark Six: winning numbers, extra number, jackpot, turnover, prize divisions,
  hot and cold numbers, odd/even split and average sum

It covers the Club's three betting products, **horse racing**, **football** and
**Mark Six**. It does not cover account balances, betting slips or placing
bets.

## Before first use

```console
$ bash scripts/ensure-binary.sh
/Users/you/.local/bin/hkjc
```

Use the printed path:

```bash
BIN="$(bash scripts/ensure-binary.sh)"
```

It checks `$HKJC_BIN`, `hkjc` on `PATH`, `scripts/install.sh`, then
`cargo build --release --locked` in the repo root.

### Install the binary directly

`npx skills add` installs the skill files. If the binary is missing, install it
with the release installer or cargo:

```bash
curl -fsSL https://raw.githubusercontent.com/hongkong-ai-labs/hongkong-jockeyclub-skill/main/scripts/install.sh | sh
# or
cargo install --git https://github.com/hongkong-ai-labs/hongkong-jockeyclub-skill --locked
```

Register it with any MCP client:

```bash
npx add-mcp "$(command -v hkjc-mcp)" --name hongkong-jockeyclub -a claude-code -a codex -a cursor -y
claude mcp add hongkong-jockeyclub -- hkjc-mcp
codex mcp add hongkong-jockeyclub -- hkjc-mcp
```

`add-mcp` detects the client configuration files in the current project; add
`-g` for a global install. `$(command -v hkjc-mcp)` keeps the entry correct
when the binary is on `PATH`; substitute an absolute path if your client does
not inherit the shell `PATH`.

## CLI

```bash
$BIN meetings                         # current meeting(s), races and post times
$BIN meetings --venue ST
$BIN card --race 7                    # runners for race 7
$BIN card --race 1 --json
$BIN odds --race 1 --types WIN,PLA,QIN
$BIN results --date 2026-11-18 --venue HV --race 4
$BIN fixtures                         # race meetings in the next 90 days
$BIN fixtures --all                   # the whole season
$BIN lookup Purton                    # find a jockey, trainer or horse
$BIN profile jockey PZ                # official profile
$BIN profile trainer TKH --json
$BIN profile horse HK_2024_K392
$BIN football --limit 20              # matches with HAD odds
$BIN football --tournament "Premier"
$BIN football-match 50079900          # full odds board
$BIN marksix --draws 10               # last 10 draws
$BIN marksix --from 2026-11-01 --to 2026-11-21
$BIN marksix-latest                   # latest draw with prize breakdown
$BIN marksix-stats --draws 100        # number frequency
$BIN mcp
$BIN update --check
```

Add `--json` to any command for structured output.

`update` detects whether the binary came from cargo, the release installer, a
source checkout or Homebrew, and updates with that same method.
`update --check` only reports.

### Venue codes

| Code | Venue |
| --- | --- |
| `ST` | Sha Tin |
| `HV` | Happy Valley |
| `CH` | Conghua |
| `S1`–`S9` | Simulcast meetings |

### What a race card contains

| Field | Meaning |
| --- | --- |
| `no`, `horse_no` | Race card number |
| `horse`, `horse_zh` | English and Chinese horse name |
| `horse_id` | HKJC horse id |
| `jockey`, `trainer` | Names as declared |
| `draw` | Barrier draw |
| `weight` | Handicap weight |
| `rating` | Current rating |
| `last_6_runs` | Finishing positions, newest first |
| `gear` | Declared gear |
| `win_odds` | Tote win odds when selling has started |
| `status` | Declared, scratched, standby, etc. |

## MCP

```json
{
  "mcpServers": {
    "hongkong-jockeyclub": {
      "command": "/absolute/path/to/hkjc-mcp",
      "args": []
    }
  }
}
```

Tools:

| Tool | Arguments |
| --- | --- |
| `hk_race_meetings` | `{ date?, venue? }` |
| `hk_race_card` | `{ race?, date?, venue? }` |
| `hk_race_odds` | `{ race?, types?, date?, venue? }` |
| `hk_race_results` | `{ date?, venue?, race? }` |
| `hk_race_fixtures` | `{ from?, to?, venue?, all? }` |
| `hk_race_profile` | `{ kind, id }` — kind is `jockey`, `trainer` or `horse` |
| `hk_race_lookup` | `{ query }` — name to profile id |
| `hk_football_matches` | `{ from?, to?, tournament?, limit? }` |
| `hk_football_match` | `{ id, types? }` |
| `hk_marksix_results` | `{ draws?, from?, to? }` |
| `hk_marksix_latest` | `{}` |
| `hk_marksix_stats` | `{ draws? }` |

## Answering patterns

- "What's on today?" → `meetings`; give venue, race numbers and post times.
- "Who's running in race 7?" → `card --race 7`; list horse, jockey, trainer,
  draw and recent form.
- "Who's favourite?" → `odds --race N --types WIN`; the `★` marks the hot
  favourite. Report odds as tote odds, not a prediction.
- "When is the next race meeting?" → `fixtures`; give date, weekday and venue.
  Use `fixtures --all` for the full season.
- "Who won race 4 at Happy Valley on 18 November?" → `results --date
  2026-11-18 --venue HV --race 4`; give the first three, then dividends.
- "Tell me about Zac Purton" → `lookup Purton`, then `profile jockey PZ`.
- "What are Arsenal's odds?" → `football --tournament "Premier"`, then
  `football-match <id>` for the full board.
- "Lucky Twenty's form" → `lookup "Lucky Twenty"`, then `profile horse <id>`.
- "What were the Mark Six numbers?" → `marksix-latest` or `marksix --draws 5`.
- "Which numbers are due?" → `marksix-stats --draws 100`. State clearly that
  draws are independent and frequency does not predict the next draw.
- "How much is the jackpot?" → `marksix-latest`; the jackpot is the published
  snowball/jackpot figure, and the first-division dividend is separate.

## Cautions

- Race meetings, declarations, odds and scratchings change constantly. Always
  call the tool for the current state; never reuse an earlier answer.
- `meetings` covers only the meeting HKJC currently publishes. For any other
  date, including future meetings, use `fixtures`.
- Historical results are only available for actual race days. A date with no
  meeting returns an error, not an empty result.
- Odds are tote/dividend odds. They are not fixed-odds prices and do not
  guarantee a return.
- Mark Six draws are random. Frequency statistics describe the past only.
- The Mark Six history endpoint returns a capped recent window even when a
  larger count is requested; report the number of draws actually analysed. An
  explicit `--from`/`--to` range must span about three months or less.
- With `--to` but no `--from`, the tool looks back 90 days.
- Football coverage follows HKJC's own list. Kick-off times are Hong Kong time,
  and in-play odds change or close during a match.
- This tool does not place bets, hold tickets or check account balances.
- Not an official HKJC channel. Check the HKJC website or a betting branch for
  confirmation.

## Source

- `https://info.cld.hkjc.com/graphql/base/` — the GraphQL API used by
  bet.hkjc.com for race cards, odds, results pools and Mark Six.
- `https://racing.hkjc.com/racing/information/English/Racing/LocalResults.aspx`
  — published local race results and dividends.
- `https://racing.hkjc.com/racing/information/English/Racing/Fixture.aspx`
  — the month-by-month fixture calendar.
- `https://racing.hkjc.com/en-us/local/information/jockeyprofile`,
  `.../trainerprofile` and `.../horse` — official profiles.

All are unofficial integrations. Responses are cached briefly in-process:
15–20 seconds for live racing data, 1 hour for fixtures and profiles, and
2 minutes for Mark Six history.
