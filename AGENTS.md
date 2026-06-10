# Jobsmith Agent Guide

Conventions for the `jobsmith` project — an AI-powered job application assistant for HeadHunter (Russia) via Kimi Code.

## Meta Principle

Before applying any rule or refactor, ask: **what problem does this solve?**
A newtype, a refactor, or an abstraction is justified only if it prevents a concrete bug,
clarifies an invariant, or removes a footgun.

## Behavioral Guidelines

- **Minimum code.** No speculative abstractions. No features beyond the request.
- **Surgical changes only.** Touch only what you must. Match existing style.
- **Goal-driven execution.** Every task needs verifiable success criteria.
- **Prefer `?` over `unwrap`/`expect` even in tests** where it keeps the test readable.
- **State assumptions explicitly.** If uncertain, ask before implementing.

## Library Contract Rules (Hard Constraints)

1. **Public API is opt-in.** Prefer `pub(crate)`. New `pub` items require a concrete external caller.
2. **Protocol facts must not go stale.** If HH API models change, update tests in the same change set.
3. **Serde roundtrip is a hard contract.** Any change to HH API models must include a serde roundtrip test.
4. **Forward-compatible fields.** Use `Option<T>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` for HH API models.
5. **Dependencies are architecture changes.** No new crate without rationale: why std/local code is not enough, transitive impact, MSRV, license.
6. **Refactors isolate mechanics from behavior.** File moves and formatting-only changes must be separate from semantic changes.
7. **MSRV is 1.85** (`kimi-wire` 0.5 requires rustc 1.85; enforced by the `msrv` CI job).

## Rust Safety Rules (Hard Constraints)

1. **`unwrap()` is banned** in production code. Use `?`, `if let`, `match`, `ok_or`.
2. **`expect()` is banned** in production code.
3. **`panic!()` is banned** in production code.
4. **`std::sync::Mutex` is banned in `async fn`.** Use `tokio::sync::Mutex`.
5. **`std::thread::sleep` is banned in `async fn`.** Use `tokio::time::sleep(...).await`.
6. **All external `Command::output().await` must have a `tokio::time::timeout`.**
7. **All `tokio::process::Command` spawn calls must set `kill_on_drop(true)` or attach to a `CancellationToken`.**

### Preconditions & Invariants

Prefer expressing preconditions in types before comments:

1. Use a specific type/newtype/parser constructor that makes invalid states unrepresentable.
2. Keep fields private when they carry invariants.
3. If the invariant cannot be encoded in the type, document it and add a `debug_assert!` next to the use.

### Tests (`#[cfg(test)]`)

`unwrap()`/`expect()` are allowed for brevity, but prefer `?` where it keeps the test readable.

## Error Handling Doctrine (Hard Constraints)

### Meta Principle

**Every error has an owner, a representation, and a consumer. Never swallow an error.**

### 1. Typed Errors Only

- **Library code uses `thiserror`.** Every public function that can fail returns a specific `JobsmithError` enum variant.
- **`anyhow` is banned from the public API.**
- **`thiserror` messages must be lowercase without trailing punctuation.**
  - Good: `#[error("connection refused")]`
  - Bad: `#[error("Connection refused.")]`

### 2. Silent Errors Are Banned

- **`let _ = ...` on `Result` is banned unless explicitly justified.**
- **Explicit ignore requires a comment.**

## HH API Compatibility

- All HH API models use `Option<T>` for optional fields.
- Use `#[serde(default, skip_serializing_if = "Option::is_none")]` for forward compatibility.
- Do not use `deny_unknown_fields` on HH API structs.
- HH API returns HTML in descriptions. Strip HTML before sending to AI.

## Secret Redaction

- The `tracing` subscriber must redact secrets (API keys, tokens) from logs.
- Any field that may contain a token or key must not be logged directly.

## Build & Test

```bash
# Run all tests
cargo test --all-features

# Lint and type-check
cargo clippy --all-targets --all-features
cargo check --all-targets --all-features

# Documentation (must build without warnings)
cargo doc --no-deps --all-features
```

## Clippy Lint Policy

Enabled lints in `src/lib.rs` via `#![warn(...)]`.

### Tier 1 — Must-fix

| Lint | What it catches |
|---|---|
| `clippy::await_holding_lock` | `std::sync::Mutex` held across `.await` |
| `clippy::dbg_macro` | `dbg!()` left in committed code |
| `clippy::wildcard_imports` | `use module::*` outside preludes/tests |
| `clippy::unused_async` | `async fn` that does not `.await` anything |

### Tier 2 — Recommended

| Lint | What it catches |
|---|---|
| `clippy::missing_panics_doc` | `expect()` / `panic!()` without doc comment |
| `clippy::cast_sign_loss` | `as u64` from signed types |

### Tier 3 — Already clean (maintain zero violations)

- Zero `TODO` / `FIXME` / `HACK` comments in production code
- All public types implement `Debug`
- Zero `unsafe` blocks
