# Changelog

All notable changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Habr Career as a second vacancy source.** `jobsmith search <query>
  --source habr` lists vacancies from Habr Career's public RSS feed, and
  `apply` auto-detects a `career.habr.com` URL and reads the vacancy's
  schema.org `JobPosting` JSON-LD — both anonymous, no token, no DOM
  scraping. Useful while hh.ru requires OAuth: the whole search → apply
  pipeline works end-to-end through Habr. Interactive TUI stays
  HeadHunter-only for now.

### Changed
- **Migrated the AI pipeline from the legacy kimi wire protocol to ACP**
  (Agent Client Protocol), the stdio protocol of Kimi Code CLI 0.14+:
  `AcpClient` replaces `KimiClient`, the `kimi-wire` dependency is gone,
  and `JobsmithError::KimiWire` is renamed to `KimiProtocol`. The ACP
  message pump auto-approves agent permission requests, rejects
  capability requests we did not declare, keeps the 10 MiB output bound
  and the 30s spawn/handshake timeout. Verified live against
  `kimi acp` (Kimi Code 0.14.0, protocol v1), including a real prompt
  roundtrip (`cargo test --lib acp_live -- --ignored`).

## [0.2.0] - 2026-06-10

Audit release: every finding of the full code review (10 important,
27 minor, 6 nits) resolved; test suite grew from 92 to 126.

### Security
- Escape `#` in AI-generated content before it reaches Typst sources,
  blocking `#read(...)`-style code injection from hostile vacancy text.
- The profile database is created `0600` and the data directory `0700`
  (unix); both previously inherited the umask.

### Added
- `jobsmith mark-applied <ID>` — mark a tracked application as submitted
  (the `applied` status was previously unreachable).
- `jobsmith search --page <N>` — 1-based result pagination.
- `jobsmith salary --role <ID> --area <ID> [--currency] [--json]` —
  online HH salary statistics (partner endpoint; the positional
  `<company>` form keeps using the local `salary_data.json`).
- CI: `msrv` (Rust 1.85) and `audit` (rustsec) jobs; `Cargo.lock` is now
  committed; documented advisory allowlist in `.cargo/audit.toml`.
- MIT `LICENSE` file; crates.io keywords/categories/package excludes.

### Changed
- **MSRV is 1.85** (was a declared-but-unbuildable 1.80: `kimi-wire` 0.5
  requires rustc 1.85).
- HH models `Employer`/`Area`/`VacancyType` use `Option<String>` for
  `id`/`name`: partial objects (hidden employers) no longer fail the
  whole search/detail parse.
- The CV prompt requests only the professional summary; profile sections
  are rendered by the template once (PDFs no longer duplicate sections).
- Retries honour the server `Retry-After` header (never sleeping less
  than the local backoff).
- Re-applying to the same vacancy reuses the existing application row
  instead of inserting duplicates; `--force` runs record a NULL fit
  score instead of a synthetic 100.
- Typst templates fall back to Helvetica Neue/Arial when Liberation
  Sans is absent (macOS).
- AI-response parsing failures surface as the new
  `JobsmithError::ResponseParse` instead of the catch-all `Process`.

### Fixed
- CI never ran: triggers matched `main` while the branch is `master`.
- `apply` ignored `--data-dir` (DB and PDFs landed in different roots)
  and created an empty nested `output/output`.
- `KimiClient::spawn`/handshake had no timeout: a hung `kimi` froze
  `apply` forever (now bounded at 30s).
- The salary short-query guard counted UTF-8 bytes, disabling itself
  for Cyrillic input.
- `REASONING` parsing no longer truncates at terminator words inside a
  sentence (line-start anchored).
- Duplicate `-e`/`-s` short flags aborted every debug-build
  `jobsmith search`.
- Setup wizard and reset confirmation no longer run blocking stdin
  reads on the async runtime; the TUI restores the terminal before
  printing a panic.

### Removed
- Unimplemented `setup --section` flag (always errored).
- Dead code: `build_interview_prep_prompt`, `SalaryLookup::list_all`,
  the never-populated `Vacancy.extra` field.
- Stray committed binaries (`test_select`, `test_writer`) and
  cargo-kimi artifacts.

## [0.1.0] - 2026-06-09

Initial release: HH search, AI-assisted apply pipeline (fit evaluation,
CV/cover drafting, review/revise) over the kimi wire protocol, Typst PDF
generation, SQLite application tracking, TUI browser, salary lookup.
