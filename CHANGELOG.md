# Changelog

All notable changes are documented here. The format follows Keep a Changelog
and the project uses semantic versioning.

## [Unreleased]

### Added

- Season fixtures, HKJC football match lists and odds boards, and official
  jockey, trainer and horse profiles with name lookup, in both the CLI and the
  MCP server (12 tools).
- Offline unit tests for profile statistics, horse form and the fixture calendar
  parsers.

### Fixed

- Fixture lists now come from the published calendar instead of the GraphQL
  endpoint, which only returned meetings already run.
- Profile parsing tolerates spaced table headers and inline `: value` cells.

## [0.1.0] - 2026-11-22

- Initial release of the `hkjc` CLI and MCP server.
- Standardized installers, Docker/Apple container instructions and CI.
