# Changelog

All notable changes are documented here. The format follows Keep a Changelog
and the project uses semantic versioning.

## [Unreleased]

### Added

- Season fixtures, HKJC football match lists and odds boards, and official
  jockey, trainer and horse profiles with name lookup, in both the CLI and the
  MCP server (18 tools).
- Jockey and trainer championship standings, barrier trial results, sectional
  times, a pace map and a Mark Six ticket checker.
- Local odds history: `hkjc odds --record` saves snapshots and `hkjc history`
  shows first/last odds and movement per combination, stored in SQLite.
- `hkjc watch` polls a live meeting and reports odds moves and scratchings,
  with optional `--record`, `--webhook` and `--telegram` delivery.
- `hkjc doctor` probes each HKJC endpoint for availability and latency.
- `--csv` output for meetings, cards, odds, results, Mark Six, fixtures,
  football, standings, trials, sectionals and odds history.
- Shell completions and a roff manual page via `hkjc completions` and `hkjc man`.
- `hkjc fixtures --next` and `hkjc fixtures --ics` for the next meeting and an
iCalendar feed.
- Offline unit tests for profile statistics, horse form, the fixture calendar
  parser, Mark Six prize divisions and the pace CSV output.

### Fixed

- Fixture lists now come from the published calendar instead of the GraphQL
  endpoint, which only returned meetings already run.
- Profile parsing tolerates spaced table headers and inline `: value` cells.
- Transient HTTP failures now retry with backoff instead of failing at once.
- `standings --season current` resolves to the live season instead of sending an
  invalid season code.

## [0.1.0] - 2026-11-22

- Initial release of the `hkjc` CLI and MCP server.
- Standardized installers, Docker/Apple container instructions and CI.
