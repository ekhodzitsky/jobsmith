# Jobsmith

AI-powered job application assistant for HeadHunter (Russia), built in Rust.

## Overview

Jobsmith automates the job application workflow:

1. **Search** — query HeadHunter API for vacancies
2. **Evaluate** — AI assesses fit between your profile and the vacancy
3. **Draft** — AI generates tailored CV and cover letter
4. **Review** — second AI pass critiques and improves the drafts
5. **Compile** — generate PDFs via Typst
6. **Track** — SQLite database tracks all applications

## Architecture

```
jobsmith/
├── src/
│   ├── cli.rs              # clap CLI definition
│   ├── commands/           # Command implementations
│   ├── error.rs            # Typed errors (thiserror)
│   ├── hh/                 # HeadHunter API client + models
│   ├── profile/            # Profile models + SQLite store
│   ├── salary/             # Salary lookup (ported from Python)
│   ├── templates/          # Typst template engine
│   └── workflow/           # Drafter-reviewer state machine + prompts
├── AGENTS.md               # Development conventions (from kimi-wire)
└── Cargo.toml
```

## Prerequisites

- Rust 1.80+
- [Typst](https://typst.app) (for PDF compilation)
- Kimi Code CLI (for AI workflow, optional for search)

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
```

## Quick Start

### 1. Set up your profile

```bash
jobsmith setup
```

Interactive wizard to enter your experience, skills, education, etc.

### 2. Search for vacancies

```bash
jobsmith search "Rust developer" --area 1 --experience between1And3
```

### 3. Apply to a vacancy

```bash
jobsmith apply https://hh.ru/vacancy/123456
```

Or by ID:

```bash
jobsmith apply 123456
```

### 4. Track applications

```bash
jobsmith list
```

### 5. Salary lookup

```bash
jobsmith salary "Яндекс" --city "Москва"
```

## Commands

| Command | Description |
|---------|-------------|
| `setup` | Interactive profile setup |
| `search` | Search HeadHunter vacancies |
| `apply` | Run AI workflow for a vacancy |
| `list` | List tracked applications |
| `salary` | Look up salary benchmarks |
| `reset` | Reset profile or application data |

## HH API Parameters

- `--area 1` — Moscow
- `--area 2` — Saint Petersburg
- `--experience noExperience` / `between1And3` / `between3And6` / `moreThan6`
- `--employment full` / `part` / `project` / `remote` / `probation`
- `--schedule fullDay` / `shift` / `flexible` / `remote` / `flyInFlyOut`

## Configuration

Profile and application data are stored in:

- macOS: `~/Library/Application Support/jobsmith/`
- Linux: `~/.local/share/jobsmith/`
- Windows: `%APPDATA%/jobsmith/`

## Development

```bash
# Run tests
cargo test --all-features

# Lint
cargo clippy --all-targets --all-features

# Format
cargo fmt

# Documentation
cargo doc --no-deps --all-features
```

## Conventions

See [AGENTS.md](./AGENTS.md) for development conventions adapted from `kimi-wire`:

- Typed errors only (`thiserror`), no `anyhow` in public API
- `unwrap()`/`expect()`/`panic!()` banned in production
- Serde roundtrip is a hard contract
- Forward-compatible fields with `Option<T>`
- `tokio::sync::Mutex` in async code
- `kill_on_drop(true)` for spawned processes

## License

MIT
