# Phase 1: Project Setup & CLI - Research

**Researched:** 2026-02-22
**Domain:** Rust CLI argument parsing, TOML configuration, MIME type detection, Cargo project scaffold
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Detect `.srt` files as a recognized type alongside video/audio/image extensions
- Silent skip for unrecognized extensions (no error output for non-media files)

### Claude's Discretion

- Config file discovery strategy (CWD-only vs XDG paths vs --config flag)
- Invalid path handling (error-and-exit vs warn-and-skip)
- Logging verbosity flags and default log level
- Startup output format
- Full list of supported media extensions

### Deferred Ideas (OUT OF SCOPE)

- Serving .srt subtitle files alongside video files — belongs in Phase 3 (HTTP streaming) or Phase 5 (ContentDirectory)
- Inline subtitle extraction from MKV containers — separate concern, future phase
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-01 | User passes one or more media directory paths as positional CLI arguments (`udlna /path1 /path2 ...`) | clap 4 `Vec<PathBuf>` positional argument pattern with `num_args = 1..` |
| CLI-03 | Server detects MIME type for video, audio, and image files from file extension; non-media files are silently skipped | mime_guess 2.0.5 `from_path().first_raw()` + custom extension filter list |
| CLI-04 | User can load configuration from an optional TOML config file searched at `./udlna.toml` then `~/.config/udlna/config.toml` | toml 1.0.3 `from_str()` + dirs 6.0.0 `config_dir()` for XDG path resolution |
| CLI-05 | CLI flags override config file values; all settings have sensible defaults requiring zero configuration | Three-layer merge: defaults → TOML → CLI flags; `Option<T>` on Config struct fields |
| CLI-07 | User can set HTTP listening port via `--port` flag (default: 8200) | clap 4 `#[arg(short, long, default_value_t = 8200)]` on `port: u16` field |
</phase_requirements>

---

## Summary

Phase 1 is a pure Rust scaffolding phase with no networking. The deliverable is a `udlna` binary that parses CLI arguments, loads an optional TOML config file with correct precedence, and classifies files by extension. All five required crates are well-established with stable APIs. The main complexity is the three-layer config merge (defaults → TOML → CLI flags) implemented with `Option<T>` fields on the config struct.

The `dirs` crate jumped from 5.x to 6.0.0 since the prior project research was written. The API for `config_dir()` and `home_dir()` is unchanged. The `toml` crate also jumped from 0.8 to 1.0.3; for deserialization use cases (reading config files with `toml::from_str()`), the API is identical — breaking changes only affect serialization and the low-level `Deserializer::new` API. The `mime_guess` crate notes explicitly that returned MIME types are not part of the stable API and can change in patch releases, so a custom override map for DLNA-critical types is mandatory regardless.

**Primary recommendation:** Scaffold the Cargo project, define the `Config` struct with `Option<T>` fields, implement the three-layer merge, wire up clap derive with `Vec<PathBuf>` positional args, implement TOML loading via `dirs::config_dir()` + CWD search, and build the extension-based MIME filter with a curated override map. Keep Phase 1 strictly to CLI/config — no networking, no scanning logic beyond extension detection.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5.60 | CLI argument parsing | De facto standard for Rust CLIs. Derive macro makes it ergonomic. Handles positional `Vec<PathBuf>`, `--port`, `--name` flags naturally with auto-generated help. |
| serde | 1.0.228 | Serialization framework | Required by toml and clap derive. `#[derive(Deserialize)]` on config struct. |
| toml | 1.0.3 | TOML config file parsing | Standard for Rust tooling. `toml::from_str()` deserializes into serde types. |
| dirs | 6.0.0 | XDG/platform config paths | `dirs::config_dir()` returns `~/.config` on Linux, `~/Library/Application Support` on macOS. Used to locate `~/.config/udlna/config.toml`. |
| mime_guess | 2.0.5 | Extension-to-MIME mapping | Maps `.mp4` → `video/mp4` etc. Fast, pure-Rust, no I/O. Must supplement with a custom override map for DLNA edge cases. |
| tracing | 0.1.44 | Structured logging | Better than env_logger for async code. Integrates with tokio. Needed from Phase 1 for startup messages and error output. |
| tracing-subscriber | 0.3.22 | Log output formatting | Provides `fmt` subscriber for terminal output. `env-filter` feature enables `RUST_LOG` level control. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing-subscriber (env-filter feature) | 0.3.22 | Runtime log level control via `RUST_LOG` env var | Always — enables `RUST_LOG=debug udlna /path` for debugging during development |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| clap derive | clap builder API | Builder is more verbose but allows dynamic argument construction. Derive is standard for fixed-schema CLIs like this one. |
| toml 1.0 | toml_edit | toml_edit preserves comments/formatting (round-trip). Overkill for read-only config parsing. |
| dirs 6.0 | etcetera, xdg | dirs is simpler, cross-platform. etcetera/xdg are alternatives for pure XDG compliance but add no value here. |
| mime_guess + override map | infer (magic bytes) | infer reads file content (slow, requires I/O). Extension-based detection is correct and fast for our no-transcode use case. |

**Installation:**
```bash
cargo add clap --features derive
cargo add serde --features derive
cargo add toml
cargo add dirs
cargo add mime_guess
cargo add tracing
cargo add tracing-subscriber --features env-filter
```

Or as a single `Cargo.toml` block:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "1"
dirs = "6"
mime_guess = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

## Architecture Patterns

### Recommended Project Structure (Phase 1 scope)

```
src/
├── main.rs          # Entry point: parse CLI, load config, merge, print startup banner
├── cli.rs           # Clap derive struct (Args) — CLI argument definitions only
├── config.rs        # Config struct + three-layer merge (defaults -> TOML -> CLI)
└── media/
    └── mime.rs      # Extension classification: is_media(), mime_for_path(), MediaKind enum
```

Files not created in Phase 1 (placeholders documented for future phases):
- `src/server.rs` — ServerState, shared Arc state (Phase 2+)
- `src/media/scanner.rs` — walkdir recursive scan (Phase 2)
- `src/http/` — hyper HTTP server (Phase 2+)
- `src/ssdp/` — UDP multicast (Phase 6)

### Pattern 1: Three-Layer Config Merge

**What:** The `Config` struct uses `Option<T>` for every user-settable field. Merge priority: hardcoded defaults (lowest) → TOML file values → CLI flags (highest). The merge function applies each layer in order, with later layers overwriting earlier ones only when `Some`.

**When to use:** Always for CLI tools with TOML config and CLI flag override.

**Example:**
```rust
// Source: pattern from docs.rs/clap and docs.rs/toml
use serde::Deserialize;

/// Hardcoded defaults
const DEFAULT_PORT: u16 = 8200;
const DEFAULT_NAME: &str = "udlna";

/// Config struct — all user-settable fields are Option<T>
/// to distinguish "not set" from "set to default value"
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub port: Option<u16>,
    pub name: Option<String>,
    pub paths: Option<Vec<std::path::PathBuf>>,
}

/// Resolved config — all fields have values after merge
#[derive(Debug)]
pub struct Config {
    pub port: u16,
    pub name: String,
    pub paths: Vec<std::path::PathBuf>,
}

impl Config {
    pub fn resolve(file: Option<FileConfig>, args: &crate::cli::Args) -> Self {
        let file = file.unwrap_or_default();
        Config {
            port: args.port
                .or(file.port)
                .unwrap_or(DEFAULT_PORT),
            name: args.name.clone()
                .or(file.name)
                .unwrap_or_else(|| DEFAULT_NAME.to_string()),
            // paths always come from CLI in Phase 1 (CLI-01 requires positional args)
            paths: args.paths.clone(),
        }
    }
}
```

### Pattern 2: clap Derive for Positional + Flag Args

**What:** Use `#[derive(Parser)]` from clap with `Vec<PathBuf>` for positional paths and `Option<T>` for optional flags (so absence can be detected during merge).

**When to use:** Always for the CLI struct.

**Example:**
```rust
// Source: docs.rs/clap/latest/clap/_derive/index.html
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "udlna",
    about = "Minimal DLNA/UPnP media server",
    version
)]
pub struct Args {
    /// One or more media directory paths to serve
    #[arg(num_args = 1..)]
    pub paths: Vec<PathBuf>,

    /// HTTP port to listen on [default: 8200]
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Friendly server name shown on DLNA clients [default: udlna]
    #[arg(short, long)]
    pub name: Option<String>,

    /// Path to TOML config file [default: ./udlna.toml or ~/.config/udlna/config.toml]
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Log verbosity: error, warn, info, debug, trace [default: info]
    #[arg(long, default_value = "info")]
    pub log_level: String,
}
```

**Note:** `paths: Vec<PathBuf>` without `#[arg(required = true)]` makes paths optional at the type level. Add `#[arg(num_args = 1..)]` to require at least one path, OR handle the empty case explicitly in main() to show usage. If no paths are provided and no config file specifies paths, print usage and exit 0 per CLI-05 (success criterion 5).

### Pattern 3: TOML Config Loading with XDG Path Search

**What:** Search two locations for the config file in order: `./udlna.toml` (CWD), then `~/.config/udlna/config.toml` (XDG). Load the first one found. If `--config` is specified explicitly, use only that path.

**When to use:** Always for CLI-04 compliance.

**Example:**
```rust
// Source: docs.rs/toml/latest/toml + docs.rs/dirs/6.0.0/dirs
use std::path::{Path, PathBuf};

pub fn find_config_file(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(path.to_owned()); // --config flag takes absolute precedence
    }
    // 1. CWD
    let cwd_config = PathBuf::from("udlna.toml");
    if cwd_config.exists() {
        return Some(cwd_config);
    }
    // 2. XDG user config dir
    if let Some(config_dir) = dirs::config_dir() {
        let xdg_config = config_dir.join("udlna").join("config.toml");
        if xdg_config.exists() {
            return Some(xdg_config);
        }
    }
    None
}

pub fn load_config(path: &Path) -> Result<crate::config::FileConfig, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let config: crate::config::FileConfig = toml::from_str(&content)?;
    Ok(config)
}
```

### Pattern 4: Extension-Based MIME Classification with Override Map

**What:** Use `mime_guess::from_path()` as a starting point, then apply a curated override map for DLNA edge cases. Classify each file as Video, Audio, Image, Subtitle, or skip (Unknown). The `.srt` extension is explicitly recognized as Subtitle.

**When to use:** Always for CLI-03 compliance, required for later phases.

**Example:**
```rust
// Source: docs.rs/mime_guess/2.0.5/mime_guess (API); override map from PITFALLS.md
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum MediaKind {
    Video,
    Audio,
    Image,
    Subtitle, // .srt and similar
}

/// Returns the media kind and the DLNA-safe MIME type string for a path,
/// or None if the file should be silently skipped.
pub fn classify(path: &Path) -> Option<(MediaKind, String)> {
    let ext = path.extension()?.to_str()?.to_lowercase();

    // Explicit override map for DLNA edge cases.
    // mime_guess "media types returned for a given extension are not part of the stable API"
    // so we lock in the DLNA-correct values here.
    let (kind, mime) = match ext.as_str() {
        // Video
        "mp4" | "m4v"        => (MediaKind::Video, "video/mp4"),
        "mkv"                => (MediaKind::Video, "video/x-matroska"),
        "avi"                => (MediaKind::Video, "video/x-msvideo"),
        "mov"                => (MediaKind::Video, "video/quicktime"),
        "ts"                 => (MediaKind::Video, "video/MP2T"),
        "m2ts" | "mts"       => (MediaKind::Video, "video/MP2T"),
        "mpg" | "mpeg"       => (MediaKind::Video, "video/mpeg"),
        "wmv"                => (MediaKind::Video, "video/x-ms-wmv"),
        "flv"                => (MediaKind::Video, "video/x-flv"),
        "ogv"                => (MediaKind::Video, "video/ogg"),
        "webm"               => (MediaKind::Video, "video/webm"),
        "3gp"                => (MediaKind::Video, "video/3gpp"),
        // Audio
        "mp3"                => (MediaKind::Audio, "audio/mpeg"),
        "flac"               => (MediaKind::Audio, "audio/flac"),
        "wav"                => (MediaKind::Audio, "audio/wav"),
        "m4a"                => (MediaKind::Audio, "audio/mp4"),
        "aac"                => (MediaKind::Audio, "audio/aac"),
        "ogg" | "oga"        => (MediaKind::Audio, "audio/ogg"),
        "wma"                => (MediaKind::Audio, "audio/x-ms-wma"),
        "opus"               => (MediaKind::Audio, "audio/opus"),
        "aiff" | "aif"       => (MediaKind::Audio, "audio/aiff"),
        // Image
        "jpg" | "jpeg"       => (MediaKind::Image, "image/jpeg"),
        "png"                => (MediaKind::Image, "image/png"),
        "gif"                => (MediaKind::Image, "image/gif"),
        "webp"               => (MediaKind::Image, "image/webp"),
        "bmp"                => (MediaKind::Image, "image/bmp"),
        "tiff" | "tif"       => (MediaKind::Image, "image/tiff"),
        // Subtitle — recognized but not served until Phase 3+
        "srt"                => (MediaKind::Subtitle, "text/srt"),
        "vtt"                => (MediaKind::Subtitle, "text/vtt"),
        // Everything else: silently skip (CLI-03)
        _                    => return None,
    };

    Some((kind, mime.to_string()))
}
```

**Key insight:** Do not rely on `mime_guess` alone for MIME type resolution. The crate docs explicitly state MIME return values are not part of the stable API. For a DLNA server where wrong MIME types cause Samsung TVs to refuse playback, locking in the correct MIME strings in a static match is mandatory.

### Pattern 5: Startup Output Format

**What:** After config is loaded and merged, print a concise startup banner so users know the server is ready. This is one of the "Claude's Discretion" items.

**Recommended approach:**
```rust
tracing::info!("udlna v{}", env!("CARGO_PKG_VERSION"));
tracing::info!("Serving paths:");
for path in &config.paths {
    tracing::info!("  {}", path.display());
}
tracing::info!("Port: {}", config.port);
tracing::info!("Server name: \"{}\"", config.name);
tracing::info!("Ready. Waiting for DLNA clients...");
```

### Pattern 6: Invalid Path Handling (Claude's Discretion)

**Recommendation: Error-and-exit** for paths that don't exist or aren't directories. Rationale: A user who provides `/bad/path` has almost certainly made a typo. Silently skipping it would leave them confused when their media doesn't appear. Fail fast with a clear message.

```rust
for path in &config.paths {
    if !path.exists() {
        eprintln!("Error: path does not exist: {}", path.display());
        std::process::exit(1);
    }
    if !path.is_dir() {
        eprintln!("Error: path is not a directory: {}", path.display());
        std::process::exit(1);
    }
}
```

### Anti-Patterns to Avoid

- **Putting paths in `FileConfig`:** While the TOML config spec (CLI-04) mentions config file support for settings, CLI-01 says paths come from positional args. In Phase 1, paths always come from the command line. The TOML file is for `port`, `name`, and optional server settings — not paths.
- **Using `String` instead of `Option<String>` in FileConfig:** Non-optional fields in `FileConfig` will fail deserialization if the TOML key is absent. All fields must be `Option<T>`.
- **Calling `dirs::home_dir()` directly:** `dirs::home_dir()` is provided but `dirs::config_dir()` is the correct function for XDG config directory (`~/.config` on Linux, `~/Library/Application Support` on macOS). Home dir is wrong for config file placement.
- **Hardcoding `~/.config` in path:** Cross-platform correctness requires `dirs::config_dir()`. macOS does NOT use `~/.config` by default.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI argument parsing | Custom argv parser | clap 4 derive | Handles short/long flags, help text, version, errors, type coercion. 1000+ edge cases. |
| TOML parsing | Custom TOML parser | toml 1.0 + serde | TOML spec has 30+ edge cases (multiline strings, datetime types, inline tables). |
| XDG config directory | Custom env var parsing | dirs 6.0 | `$XDG_CONFIG_HOME` override, macOS path differences, Windows `AppData\Roaming`. |
| Extension to MIME mapping | Custom extension list | mime_guess as starting point | mime_guess covers 1000+ MIME types. Still need override map for DLNA edge cases (10 entries). |
| Log filtering by level | Custom level filter | tracing-subscriber env-filter | `RUST_LOG=debug` granular per-module filtering. Non-trivial to implement correctly. |

**Key insight:** Phase 1 has no novel problems. Every sub-problem (CLI parsing, TOML reading, path detection, MIME mapping) has a well-established Rust crate. The work is wiring them together correctly with the three-layer merge pattern.

---

## Common Pitfalls

### Pitfall 1: `Vec<PathBuf>` Positional Args Without `num_args`

**What goes wrong:** `paths: Vec<PathBuf>` in a clap derive struct without `#[arg(num_args = 1..)]` will not enforce at least one path. Clap treats `Vec<T>` as optional (zero or more). Users can accidentally run `udlna --port 9000` with no paths, getting an unclear error downstream.

**Why it happens:** Clap's automatic behavior for `Vec<T>` implies `required = false`. The constraint "at least one path" must be stated explicitly.

**How to avoid:** Add `#[arg(num_args = 1..)]` to the paths field. This enforces at least one argument and generates a clear error message if absent.

**Warning signs:** `udlna` with no paths exits cleanly with no error — when it should either error or show usage.

### Pitfall 2: toml 1.0 `#[serde(deny_unknown_fields)]` Blocks Forward Compatibility

**What goes wrong:** Adding `#[serde(deny_unknown_fields)]` to `FileConfig` means any unrecognized TOML key causes a parse error. This breaks if a user has a config file from a newer version of udlna with fields Phase 1 doesn't know about.

**Why it happens:** Developers add `deny_unknown_fields` defensively to catch typos.

**How to avoid:** Do NOT use `#[serde(deny_unknown_fields)]` on `FileConfig`. Unknown keys should be silently ignored. This is idiomatic for config files where the schema evolves. (If catching typos is desired, implement a warning-only approach with a custom deserializer in a later phase.)

**Warning signs:** Config file from a new version causes cryptic "unknown field" errors on an old binary.

### Pitfall 3: dirs 6.0 API Change from 5.x

**What goes wrong:** Prior project research referenced `dirs = "^5.0"`. The current version is `dirs = "6.0.0"`. The jump to 6.0 is a major version — there may be breaking changes.

**Why it happens:** The prior research was based on early 2025 training data; the library advanced.

**How to avoid:** The key functions `config_dir()` and `home_dir()` exist in 6.0.0 (verified via docs.rs). Check the changelog for any behavioral changes. The API surface we use is small: just `dirs::config_dir()`. Pin to `dirs = "6"` in Cargo.toml.

**Warning signs:** Compilation failure if relying on a removed function from 5.x. As of this research, `config_dir()` exists in 6.0.0.

### Pitfall 4: No-Args Case Produces Bad UX

**What goes wrong:** Running `udlna` with no arguments (and no config file) should produce a helpful usage message (CLI-05 success criterion 5). By default clap will print "error: the following required arguments were not provided: `<paths>...`" which is correct but not "helpful with sensible defaults documented."

**Why it happens:** clap's default error for missing required args is terse.

**How to avoid:** Either (a) make `paths` optional in the clap struct and handle the empty case explicitly to print `--help` output, or (b) use `#[command(arg_required_else_help = true)]` on the Args struct to automatically print help when no args are given.

**Recommended:** `#[command(arg_required_else_help = true)]` — this prints the full help text (with defaults documented in field doc comments) when called with no arguments, which satisfies CLI-05.

```rust
#[derive(Parser, Debug)]
#[command(
    name = "udlna",
    about = "Minimal DLNA/UPnP media server — point at media, works instantly",
    version,
    arg_required_else_help = true,
)]
pub struct Args { ... }
```

### Pitfall 5: MIME Override Map Omissions Break Later Phases

**What goes wrong:** If Phase 1 defines an incomplete extension list, later phases (especially ContentDirectory, Phase 5) inherit the gaps. Files not recognized in Phase 1's `classify()` function are silently skipped — they won't appear in the DLNA library.

**Why it happens:** Developers test with their own files and miss formats they don't personally use.

**How to avoid:** The extension list in Pattern 4 (Code Examples) covers all common formats. Add `srt` explicitly (per locked decision). For now, omitting exotic formats (HEVC-specific containers, etc.) is fine — the list can be extended in later phases.

**Warning signs:** A user's `.avi` or `.m4a` files don't appear in the server. Check the classify function.

### Pitfall 6: Logging Infrastructure Not Set Up in Phase 1

**What goes wrong:** Skipping tracing setup in Phase 1 means later phases must retrofit it. The `tracing` macros (`tracing::info!`, `tracing::debug!`) emit nothing if no subscriber is initialized.

**Why it happens:** "I'll add logging later."

**How to avoid:** Initialize `tracing_subscriber` in `main()` before any other code runs. Even a minimal setup with `tracing_subscriber::fmt::init()` is sufficient for Phase 1 — the `env-filter` feature enables `RUST_LOG` control without additional code.

```rust
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();
    // ... rest of main
}
```

---

## Code Examples

Verified patterns from official sources:

### Main Entry Point Structure

```rust
// main.rs — Phase 1 entry point
use clap::Parser;

mod cli;
mod config;
mod media;

fn main() {
    // 1. Initialize logging first (tracing-subscriber)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    // 2. Parse CLI args
    let args = cli::Args::parse();

    // 3. Find and load config file
    let file_config = config::find_config_file(args.config.as_deref())
        .and_then(|path| {
            match config::load_config(&path) {
                Ok(cfg) => {
                    tracing::debug!("Loaded config from {}", path.display());
                    Some(cfg)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config file {}: {}", path.display(), e);
                    None
                }
            }
        });

    // 4. Merge: defaults <- file_config <- CLI args
    let config = config::Config::resolve(file_config, &args);

    // 5. Validate paths
    for path in &config.paths {
        if !path.exists() || !path.is_dir() {
            eprintln!("Error: not a valid directory: {}", path.display());
            std::process::exit(1);
        }
    }

    // 6. Print startup banner
    tracing::info!("udlna v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Serving {} path(s) on port {}", config.paths.len(), config.port);
    for path in &config.paths {
        tracing::info!("  {}", path.display());
    }

    // Phase 1 ends here — Phase 2 adds the tokio runtime and HTTP server
    tracing::info!("(Phase 1 stub — server not yet implemented)");
}
```

### clap Args Struct

```rust
// Source: docs.rs/clap/latest/clap/_derive/index.html
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "udlna",
    about = "Minimal DLNA/UPnP media server — `udlna /path/to/media` and it works",
    long_about = None,
    version,
    arg_required_else_help = true,
)]
pub struct Args {
    /// One or more directories containing media files to serve
    #[arg(num_args = 1..)]
    pub paths: Vec<PathBuf>,

    /// HTTP port to listen on
    #[arg(short, long, default_value = "8200")]
    pub port: Option<u16>,

    /// Friendly server name shown on DLNA client device lists
    #[arg(short, long)]
    pub name: Option<String>,

    /// Path to TOML config file (overrides default search locations)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Log level: error, warn, info, debug, trace
    #[arg(long, default_value = "info")]
    pub log_level: String,
}
```

**Note on port:** Use `Option<u16>` (not `u16`) so absence can be detected during three-layer merge. The `default_value = "8200"` is set in the help text for documentation but the actual default is applied in `Config::resolve()`.

### toml Deserialization

```rust
// Source: docs.rs/toml/latest/toml
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default, Debug)]
pub struct FileConfig {
    pub port: Option<u16>,
    pub name: Option<String>,
    // Note: paths NOT included — paths come from CLI (CLI-01)
}

// Usage:
// let config: FileConfig = toml::from_str(&file_contents)?;
```

Example `udlna.toml`:
```toml
port = 9000
name = "Living Room Media"
```

### MIME Classification in Practice

```rust
// In scanner (Phase 2), used like:
use crate::media::mime::classify;
use std::path::Path;

fn process_file(path: &Path) {
    match classify(path) {
        Some((kind, mime)) => {
            tracing::debug!("Recognized {:?}: {} ({})", kind, path.display(), mime);
            // Add to media library
        }
        None => {
            tracing::trace!("Skipping non-media file: {}", path.display());
            // Silent skip per CLI-03
        }
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `dirs = "^5.0"` (from prior research) | `dirs = "6.0.0"` | Between early 2025 and now | Major version bump; `config_dir()` and `home_dir()` API unchanged, verify changelog for behavioral changes |
| `toml = "^0.8"` (from prior research) | `toml = "1.0.3"` | 2024-2025 | Breaking changes in serializer and `Deserializer::new` API; `toml::from_str()` deserialization pattern unchanged |
| clap 3.x | clap 4.5.60 | 2022 (clap 4.0), stable since | `#[command(...)]` replaces `#[clap(...)]` attribute macro; derive API is stable |
| `env_logger` + `log` crate | `tracing` + `tracing-subscriber` | Ongoing shift ~2021-2023 | tracing is now the standard for async Rust; provides spans and structured events |

**Deprecated/outdated from prior project research:**
- `dirs = "^5.0"`: Use `dirs = "6"` instead. Minor concern — the core API we use is stable.
- `toml = "^0.8"`: Use `toml = "1"` instead. Deserialization API unchanged; only affects code using old serializer or low-level `Deserializer::new`.

---

## Open Questions

1. **Does `dirs::config_dir()` on macOS return `~/Library/Application Support` or `~/.config`?**
   - What we know: `dirs` follows platform conventions. On macOS, `config_dir()` returns `~/Library/Application Support` per Apple guidelines.
   - What's unclear: Whether the user expects this macOS path or a Linux-style `~/.config/udlna/config.toml`.
   - Recommendation: Document both paths in `--help` output. The `dirs::config_dir()` behavior is correct and cross-platform. macOS users who prefer `~/.config` can use `--config` flag explicitly.

2. **Should `paths` field be allowed in `udlna.toml`?**
   - What we know: CLI-01 requires positional CLI arguments. CLI-04 allows TOML config.
   - What's unclear: Whether TOML-configured paths are in scope for Phase 1.
   - Recommendation: Do NOT include `paths` in `FileConfig` for Phase 1. Paths are CLI-only per CLI-01's framing ("positional CLI arguments"). If desired later, add `paths` to `FileConfig` in a future phase.

3. **How should the `--log-level` flag interact with `RUST_LOG` env var?**
   - What we know: tracing-subscriber's env-filter feature supports `RUST_LOG`. Clap can also define a `--log-level` flag.
   - What's unclear: Which takes precedence when both are set.
   - Recommendation: Use `RUST_LOG` as the canonical override (`EnvFilter::from_env("RUST_LOG")`). If `RUST_LOG` is unset, default to `info`. Deprecate or omit the `--log-level` flag for simplicity — `RUST_LOG=debug udlna /path` is the standard Rust idiom.

---

## Sources

### Primary (HIGH confidence)

- `cargo search clap` — `clap = "4.5.60"` verified 2026-02-22
- `cargo search serde` — `serde = "1.0.228"` verified 2026-02-22
- `cargo search toml` — `toml = "1.0.3+spec-1.1.0"` verified 2026-02-22
- `cargo search dirs` — `dirs = "6.0.0"` verified 2026-02-22
- `cargo search mime_guess` — `mime_guess = "2.0.5"` verified 2026-02-22
- `cargo search tracing` — `tracing = "0.1.44"` verified 2026-02-22
- `cargo search tracing-subscriber` — `tracing-subscriber = "0.3.22"` verified 2026-02-22
- `cargo search tokio` — `tokio = "1.49.0"` verified 2026-02-22 (for Phase 2+ planning)
- `cargo info dirs` — confirmed `config_dir()` and `home_dir()` exist in dirs 6.0.0
- https://docs.rs/clap/latest/clap/_derive/index.html — `Vec<PathBuf>` positional arg pattern, `num_args`, `arg_required_else_help`
- https://docs.rs/toml/latest/toml/ — `toml::from_str()` deserialization pattern confirmed unchanged from 0.8
- https://docs.rs/dirs/6.0.0/dirs/ — `config_dir()` and `home_dir()` API confirmed present
- https://docs.rs/mime_guess/2.0.5/mime_guess/ — MIME types "not part of stable API" warning confirmed; `from_path().first_raw()` API documented
- `.planning/research/STACK.md` — prior project-level research, HIGH confidence for Rust ecosystem choices
- `.planning/research/PITFALLS.md` — MIME type override map data, MEDIUM confidence

### Secondary (MEDIUM confidence)

- WebSearch "clap 4 derive Vec PathBuf positional arguments" — multiple sources confirm `Vec<PathBuf>` without `#[arg]` works; `num_args = 1..` for required minimum
- WebSearch "toml crate 1.0 vs 0.8 breaking changes" — breaking changes confirmed for serializer and Deserializer::new; `from_str()` deserialization unchanged

### Tertiary (LOW confidence)

- macOS `config_dir()` path behavior — inferred from `dirs` documentation and platform conventions; not directly verified in this session

---

## Metadata

**Confidence breakdown:**
- Standard stack (crate selection): HIGH — all versions verified via `cargo search` and `cargo info` against live crates.io
- Architecture (three-layer merge pattern): HIGH — standard Rust CLI idiom, verified against clap and toml official docs
- MIME override map: MEDIUM — extension-to-MIME mappings from prior project PITFALLS research; DLNA edge cases (MKV variants) remain MEDIUM until tested with real devices
- Pitfalls: HIGH — based on verified clap/toml/dirs API behavior plus prior project research

**Research date:** 2026-02-22
**Valid until:** 2026-03-24 (30 days — all libraries in this phase are stable/slow-moving)

---

*Phase: 01-project-setup-cli*
*Written for: gsd-planner consumption*
