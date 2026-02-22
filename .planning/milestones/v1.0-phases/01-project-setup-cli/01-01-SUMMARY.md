---
phase: 01-project-setup-cli
plan: "01"
subsystem: infra
tags: [rust, cargo, clap, mime, dlna]

# Dependency graph
requires: []
provides:
  - "Compilable Cargo project with all Phase 1 dependencies declared (clap 4, serde 1, toml 1, dirs 6, mime_guess 2, tracing 0.1, tracing-subscriber 0.3)"
  - "clap derive Args struct with Vec<PathBuf> paths (num_args=1..), Option<u16> port, Option<String> name, Option<PathBuf> config"
  - "MediaKind enum and classify() function covering 36 file extensions with DLNA-correct MIME strings"
  - "Locked decision enforced: .srt classified as Subtitle, not skipped"
affects:
  - "02-config-merge"
  - "03-http-server"
  - "04-media-scanner"
  - "all subsequent phases that use Args or classify()"

# Tech tracking
tech-stack:
  added:
    - "clap 4.5.x (derive feature) — CLI argument parsing"
    - "serde 1.x (derive feature) — serialization framework for TOML config"
    - "toml 1.0.x — TOML config file parsing"
    - "dirs 6.0.x — platform-specific config directories"
    - "mime_guess 2.x — MIME detection (declared; not used in classify() per research guidance)"
    - "tracing 0.1.x — structured logging"
    - "tracing-subscriber 0.3.x (env-filter) — RUST_LOG log level control"
  patterns:
    - "Extension-to-MIME mapping via static match on lowercased extension string"
    - "Option<T> for CLI flags to enable three-layer config merge in Plan 02"
    - "Silent skip for unrecognized files at classify() layer (no logging)"

key-files:
  created:
    - "Cargo.toml — project manifest with all Phase 1 dependencies"
    - "src/cli.rs — Args struct (pub paths: Vec<PathBuf>, pub port: Option<u16>, pub name: Option<String>, pub config: Option<PathBuf>)"
    - "src/media/mod.rs — media module declaration"
    - "src/media/mime.rs — MediaKind enum and classify() with 8 unit tests"
    - "src/config.rs — empty stub for Plan 02"
  modified:
    - "src/main.rs — integrates Args::parse() so --help and arg_required_else_help work"

key-decisions:
  - "Use Option<u16> for --port (not u16 with default) so absence is detectable during config merge in Plan 02"
  - "No --log-level flag; use RUST_LOG env var exclusively per research recommendation"
  - "classify() uses explicit static match strings, not mime_guess return values (API not stable)"
  - "#![allow(dead_code)] on src/media/mod.rs to suppress unused warnings on public API stubs"
  - ".srt classified as Subtitle with MIME text/srt (LOCKED DECISION from context)"

patterns-established:
  - "Clap arg_required_else_help=true: running udlna with no args prints full help"
  - "MIME mapping pattern: lowercase extension string matched in static match, returns Option<(MediaKind, &'static str)>"

requirements-completed:
  - CLI-01
  - CLI-03
  - CLI-07

# Metrics
duration: 3min
completed: 2026-02-22
---

# Phase 1 Plan 01: Project Setup & CLI Summary

**Cargo project scaffolded with clap 4 derive CLI (positional paths + optional port/name/config), and extension-based MIME classifier returning DLNA-correct types for 36 extensions including .srt as Subtitle**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-22T20:42:30Z
- **Completed:** 2026-02-22T20:45:33Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Cargo.toml declares all 7 Phase 1 dependencies (clap, serde, toml, dirs, mime_guess, tracing, tracing-subscriber) at the specified major versions; cargo build succeeds
- src/cli.rs: Args struct with Vec<PathBuf> positional paths (num_args=1..), Option<u16> port, Option<String> name, Option<PathBuf> config; arg_required_else_help=true makes --help work with no args
- src/media/mime.rs: MediaKind enum (Video, Audio, Image, Subtitle), classify() covering 15 video / 11 audio / 8 image / 2 subtitle extensions using DLNA-correct MIME strings; 8 unit tests all pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Initialize Cargo project and declare all Phase 1 dependencies** - `9f88196` (feat)
2. **Task 2: Implement CLI Args struct and MIME classification module** - `26b3085` (feat)

**Plan metadata:** (docs commit — see Final Commit)

## Files Created/Modified

- `Cargo.toml` — project manifest with edition 2021, all 7 Phase 1 dependencies
- `src/main.rs` — minimal stub integrating Args::parse() to enable --help
- `src/cli.rs` — clap derive Args struct with all 4 flags
- `src/config.rs` — empty stub ("Implemented in Plan 02")
- `src/media/mod.rs` — module declaration with #![allow(dead_code)]
- `src/media/mime.rs` — MediaKind enum, classify() function, 8 unit tests
- `Cargo.lock` — locked dependency resolution

## Decisions Made

- Option<u16> for --port instead of u16 with default: enables three-layer config merge in Plan 02 to detect flag presence
- No --log-level flag: use RUST_LOG env var exclusively (simpler UX per research)
- Static match strings in classify() rather than mime_guess return values: mime_guess API is not stable per research guidance
- #![allow(dead_code)] on media module: suppresses unused warnings on public API intentionally stubbed for future plans
- .srt classified as Subtitle (LOCKED DECISION from context — must not be skipped)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added Args::parse() call in main.rs**
- **Found during:** Task 2 verification
- **Issue:** Plan said "leave main.rs as a minimal stub" but without Args::parse(), `cargo run -- --help` just printed the stub println, not clap help output. The verification criterion explicitly requires `--help` to show usage.
- **Fix:** Added `use clap::Parser;` and `let _args = cli::Args::parse();` in main.rs
- **Files modified:** src/main.rs
- **Verification:** `cargo run -- --help` now shows full clap-generated usage with [PATHS]... and --port documented
- **Committed in:** 26b3085 (Task 2 commit)

**2. [Rule 2 - Missing Critical] Added #![allow(dead_code)] to suppress build warnings**
- **Found during:** Task 2 verification
- **Issue:** Plan requires "cargo build exits 0 with no errors or warnings" but MediaKind and classify() are unused by main.rs (stub phase), generating dead_code warnings
- **Fix:** Added #![allow(dead_code)] at module level in src/media/mod.rs
- **Files modified:** src/media/mod.rs
- **Verification:** cargo build produces zero warnings
- **Committed in:** 26b3085 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 2 - missing critical for verification requirements)
**Impact on plan:** Both fixes required to satisfy explicit verification criteria. No scope creep.

## Issues Encountered

- toml v"1" and dirs v"6" were available on crates.io (verified via cargo info) despite being recent major versions not in training data snapshot

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Args struct ready for Plan 02 (config merge): paths, port, name, config all Option<T> or Vec<T>
- classify() ready for Plan 04 (media scanner): covers all required extensions, returns None silently for unrecognized
- .srt subtitle tracking locked in — Plan 03/05 can rely on Subtitle classification being present

---
*Phase: 01-project-setup-cli*
*Completed: 2026-02-22*
