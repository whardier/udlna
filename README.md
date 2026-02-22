# udlna

A minimal DLNA/UPnP media server. Point it at a folder, stream to your TV.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## About

`udlna` scans a media directory and advertises it as a DLNA/UPnP server on your local network. No transcoding, no database, no daemon to manage — just start it when you need it and stop it when you're done. Verified working with Samsung Smart TVs and Xbox Series X.

## Features

- **Zero-config defaults** — one argument to get started
- **Broad format support** — video (MP4, MKV, WebM), audio (MP3, FLAC, AAC, OGG, WAV), images (JPEG, PNG)
- **HTTP byte-range streaming** — clients can seek within files
- **SSDP auto-discovery** — devices on your network find the server automatically
- **Stable identity** — UUID derived from hostname + server name; consistent across restarts
- **No transcoding** — files are served as-is; your client handles codec compatibility
- **Optional TOML config** — persist settings without repeating flags

## Requirements

- Rust 1.93.0+ (to build from source)
- Linux or macOS

## Installation

```bash
git clone https://github.com/whardier/udlna
cd udlna
cargo build --release
```

The binary will be at `target/release/udlna`.

## Usage

```bash
# Serve a directory on the default port (8200)
udlna /path/to/media

# Custom port and server name
udlna /path/to/media --port 9000 --name "Living Room"

# Bind to localhost only (disables network advertisement)
udlna /path/to/media --localhost
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `-p, --port <PORT>` | `8200` | HTTP listen port |
| `-n, --name <NAME>` | `udlna@{hostname}` | Friendly name shown on DLNA devices |
| `-c, --config <FILE>` | — | Path to TOML config file |
| `--localhost` | off | Bind to 127.0.0.1 only |

### Config file

`udlna` looks for a config file in this order:

1. `--config <path>` (explicit flag)
2. `./udlna.toml` (current directory)
3. `~/.config/udlna/config.toml`

Example `udlna.toml`:

```toml
port = 9000
name = "Living Room"
localhost = false
```

CLI flags take precedence over config file values.

## Compatibility notes

`udlna` serves files as-is — there is no transcoding. If a client cannot play a particular format, it is a codec compatibility issue on the client side. Tested devices:

| Device | Status |
|--------|--------|
| Samsung Smart TV | Verified |
| Xbox Series X | Verified |

## Contributing

Contributions are welcome. Please open an issue before submitting large changes.

### Development setup

```bash
git clone https://github.com/whardier/udlna
cd udlna
cargo build
```

### Running tests

```bash
cargo test
```

### Project structure

| Module | Purpose |
|--------|---------|
| `src/cli.rs` | Argument parsing |
| `src/config.rs` | Config file loading and CLI/file/default merge |
| `src/scanner.rs` | Recursive media discovery and metadata extraction |
| `src/http.rs` | HTTP server and byte-range streaming |
| `src/ssdp.rs` | SSDP advertisement and M-SEARCH responses |
| `src/upnp/` | ContentDirectory, ConnectionManager, device description |

## License

`udlna` is licensed under the MIT license. See [`LICENSE`](LICENSE) for details.
