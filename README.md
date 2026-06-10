# Jobsmith

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

AI-powered job application assistant for HeadHunter. Automates search, evaluation, CV generation, and PDF compilation — so you can focus on interviews, not paperwork.

## What it does

- **Smart Search** — query HeadHunter with fine-grained filters
- **AI Fit Score** — evaluates how well a vacancy matches your profile
- **Auto-CV & Cover Letter** — tailored to each position via AI pipeline
- **PDF Compilation** — professional output via Typst
- **Application Tracking** — SQLite-backed history with status management

## Quick Start

```bash
# 1. Set up your profile
jobsmith setup

# 2. Search for vacancies
jobsmith search "Rust developer" --area 1

# 3. Apply with AI-generated documents
jobsmith apply https://hh.ru/vacancy/123456

# 4. Track everything
jobsmith list
```

## Installation

```bash
cargo install --path .
```

Requires [Typst](https://typst.app) for PDF generation.

## License

MIT
