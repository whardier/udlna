---
phase: 01-project-setup-cli
plan: "02"
subsystem: cli
tags: [rust, clap, serde, toml, dirs, tracing, tracing-subscriber]

# Dependency graph
requires:
  - phase: 01-project-setup-cli/01-01
    provides: cli.rs (Args with Option<u16> port, Option<String> name, Vec<PathBuf> paths, Option<PathBuf> config), media module stub
provides:
  - src/config.rs: FileConfig (TOML deserialization), Config (resolved), find_config_file(), load_config(), Config::resolve()
  - src/main.rs: Fully wired entry point with tracing init, CLI parse, config load, merge, path validation, startup banner
  - Three-layer config merge: defaults (8200/"udlna") -> TOML file -> CLI flags (highest priority)
  - Fail-fast path validation: eprintln + exit(1) for nonexistent or non-directory paths
affects:
  - 02-media-scanning (reads config.paths to scan directories)
  - 03-http-server (reads config.port to bind listener)
  - all subsequent phases (Config struct is the system-wide config carrier)

# Tech tracking
tech-stack:
  added:
    - serde (derive) - TOML deserialization via FileConfig
    - toml 1.x - TOML file parsing
    - dirs 6.x - XDG config_dir() for platform-native config search path
    - tracing + tracing-subscriber (env-filter) - structured logging (wired to main)
  patterns:
    - Three-layer config merge: args.port.or(file.port).unwrap_or(DEFAULT_PORT)
    - Option<T> fields in FileConfig allow absent TOML keys without parse failure
    - No #[serde(deny_unknown_fields)] on FileConfig for forward compatibility
    - RUST_LOG env var for log level (no --log-level CLI flag)
    - eprintln! + process::exit(1) for fatal user errors (not tracing)

key-files:
  created:
    - src/config.rs
  modified:
    - src/main.rs

key-decisions:
  - "FileConfig uses Option<T> fields only — absent TOML keys never fail deserialization"
  - "No #[serde(deny_unknown_fields)] on FileConfig — unknown future TOML keys silently ignored for forward compatibility"
  - "Malformed config file logs warn and continues — startup not blocked when user passed valid CLI paths"
  - "Path validation uses eprintln!/exit(1) not tracing — fatal user errors go to stderr directly"
  - "RUST_LOG env var exclusively for log level — no --log-level flag"

patterns-established:
  - "Config merge pattern: args.field.or(file.field).unwrap_or(DEFAULT)"
  - "Config search: explicit path > CWD/udlna.toml > XDG config_dir()/udlna/config.toml > None"

requirements-completed: [CLI-04, CLI-05]

# Metrics
duration: 2min
completed: 2026-02-22
---

# Phase 1 Plan 02: Config Module Summary

**TOML config loading with XDG path search and three-layer merge (defaults -> TOML -> CLI flags) wired into a complete entry point with tracing, path validation, and startup banner**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-02-22T20:48:51Z
- **Completed:** 2026-02-22T20:50:15Z
- **Tasks:** 2 completed
- **Files modified:** 2

## Accomplishments

- Implemented FileConfig (serde Deserialize, all Option<T>, no deny_unknown_fields) and Config (resolved concrete types) in src/config.rs
- Implemented Config::resolve() with three-layer merge pattern: hardcoded defaults -> TOML file values -> CLI flags
- Implemented find_config_file() with CWD-first XDG search using dirs::config_dir()
- Wired src/main.rs: tracing init, clap parse, config file search/load, three-layer merge, fail-fast path validation, startup banner
- All 14 tests pass (8 MIME + 6 config), cargo build exits 0 with no warnings

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement config module with TOML loading and three-layer merge** - `5010c51` (feat)
2. **Task 2: Wire main.rs — logging, CLI parse, config load, path validation, startup banner** - `714311b` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `/Users/spencersr/tmp/udlna/src/config.rs` - FileConfig (TOML-deserializable, Option<T> fields), Config (resolved), Config::resolve(), find_config_file(), load_config(), 6 unit tests
- `/Users/spencersr/tmp/udlna/src/main.rs` - Fully wired entry point: tracing init, CLI parse, config file load with warn-on-error, Config::resolve(), path validation with exit(1), startup banner

## Decisions Made

- FileConfig has no `#[serde(deny_unknown_fields)]` — future TOML keys added in later phases must not break older configs (forward compatibility, per plan research)
- Malformed TOML config logs a warning and continues with defaults rather than aborting startup — user's CLI paths are still valid
- Path validation uses `eprintln!` + `std::process::exit(1)` rather than tracing, since this is a fatal user-facing error that must be visible regardless of RUST_LOG setting

## Deviations from Plan

None - plan executed exactly as written.

The test count was 14 (not 13 as mentioned in plan task 2 verify), because plan 01-01 delivered 8 MIME tests, not 7. This is a documentation discrepancy in the plan, not a deviation.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 1 is complete. The `udlna` binary satisfies all five phase success criteria:
  - Defaults to port 8200 with no config
  - CLI flag (--port) overrides TOML and defaults
  - TOML file (./udlna.toml or XDG path) overrides defaults
  - CLI flag overrides TOML (three-layer merge)
  - Invalid paths print error and exit 1
- Ready for Phase 2 (media scanning): Config struct carries config.paths for directory iteration

---
*Phase: 01-project-setup-cli*
*Completed: 2026-02-22*

## Self-Check: PASSED

- FOUND: src/config.rs
- FOUND: src/main.rs
- FOUND: .planning/phases/01-project-setup-cli/01-02-SUMMARY.md
- FOUND commit: 5010c51 (feat(01-02): implement config module)
- FOUND commit: 714311b (feat(01-02): wire main.rs)
