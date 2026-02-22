use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "udlna",
    about = "Minimal DLNA/UPnP media server — `udlna /path/to/media` and it works",
    long_about = None,
    version = env!("GIT_VERSION"),
    arg_required_else_help = true,
)]
pub struct Args {
    /// One or more directories containing media files to serve
    #[arg(num_args = 1..)]
    pub paths: Vec<PathBuf>,

    /// HTTP port to listen on [default: 8200]
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Friendly server name shown on DLNA client device lists [default: udlna]
    #[arg(short, long)]
    pub name: Option<String>,

    /// Path to TOML config file (overrides default search: ./udlna.toml, ~/.config/udlna/config.toml)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Bind to localhost only (127.0.0.1) instead of all interfaces (0.0.0.0 + :::)
    #[arg(long)]
    pub localhost: bool,
}
