use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use axum::Router;
use clap::Parser;

use udlna::{cli, config, http, media, ssdp};
use udlna::media::library::MediaLibrary;

/// Set to true once the first Ctrl+C is received. Second Ctrl+C force-exits.
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

/// Wait for the first Ctrl+C (graceful shutdown).
/// On first Ctrl+C, sets SHUTTING_DOWN and returns.
/// On second Ctrl+C (during shutdown wait), force-exits immediately.
async fn wait_for_shutdown() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    if SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
        eprintln!("\nudlna: forced exit");
        std::process::exit(1);
    }
    // first Ctrl+C: proceed with graceful shutdown
}

/// Derive a stable UUID v5 from hostname + server name using DNS namespace.
/// Combines both inputs so the UUID is stable across restarts on the same machine
/// with the same name, but changes if the name changes on a different machine or
/// if the user explicitly customizes the name.
fn build_server_uuid(hostname: &str, server_name: &str) -> String {
    let seed = format!("{}\x00{}", hostname, server_name);
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, seed.as_bytes()).to_string()
}

/// Acquire the OS hostname safely, falling back to "udlna" if unavailable.
fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|os| os.into_string().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "udlna".to_string())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let args = cli::Args::parse();

    let file_config = config::find_config_file(args.config.as_deref())
        .and_then(|path| {
            match config::load_config(&path) {
                Ok(cfg) => {
                    tracing::debug!("Loaded config from {}", path.display());
                    Some(cfg)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse config file: {}", e);
                    None
                }
            }
        });

    let config = config::Config::resolve(file_config, &args);

    for path in &config.paths {
        if !path.exists() {
            eprintln!("error: path does not exist: {}", path.display());
            std::process::exit(1);
        }
        if !path.is_dir() {
            eprintln!("error: not a directory: {}", path.display());
            std::process::exit(1);
        }
    }

    // Derive stable UUID v5 from hostname + server name (Phase 8: replaces random UUID v4).
    let raw_hostname = get_hostname();
    let server_uuid = build_server_uuid(&raw_hostname, &config.name);

    // Startup banner: single line with name, UUID, and port.
    tracing::info!(
        "udlna \"{}\" (uuid: {}) on port {}",
        config.name,
        server_uuid,
        config.port
    );
    tracing::info!("Scanning media directories:");
    for path in &config.paths {
        tracing::info!("  {}", path.display());
    }

    // Synchronous scan -- blocks the thread; acceptable since server has not started yet
    let library = media::scanner::scan(&config.paths);

    // LOCKED: zero media files found must be an error exit (not a silent empty server)
    if library.items.is_empty() {
        eprintln!("error: no media files found in the provided paths -- exiting");
        std::process::exit(1);
    }

    // Wrap in Arc<RwLock<>> for thread-safe sharing across route handlers.
    // Write-once at startup, then read-only. std::sync::RwLock is safe here.
    // If Phase 6+ needs writes (OPER-01 SIGHUP rescan), switch to tokio::sync::RwLock.
    let library = Arc::new(RwLock::new(library));
    let state = http::state::AppState {
        library: Arc::clone(&library),
        server_uuid: server_uuid.clone(),
        server_name: config.name.clone(),
    };
    let app = http::build_router(state);

    if config.localhost {
        run_localhost(config.port, config.name, server_uuid, library, app).await;
    } else {
        run_dual_stack(config.port, config.name, server_uuid, library, app).await;
    }
}

/// Run a localhost-only HTTP + SSDP server and wait for graceful shutdown.
async fn run_localhost(
    port: u16,
    server_name: String,
    server_uuid: String,
    library: Arc<RwLock<MediaLibrary>>,
    app: Router,
) {
    let addr = format!("127.0.0.1:{}", port);
    tracing::info!(
        "Serving {} media items on http://{} (localhost only)",
        library.read().expect("library lock poisoned").items.len(),
        addr
    );
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("error: failed to bind {}: {}", addr, e);
            std::process::exit(1);
        });

    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(4);

    // SSDP task: spawned after HTTP listener is bound.
    let ssdp_config = ssdp::service::SsdpConfig {
        device_uuid: server_uuid,
        http_port: port,
        server_name,
    };
    let ssdp_shutdown_rx = shutdown_tx.subscribe();
    let ssdp_task = tokio::spawn(ssdp::service::run(ssdp_config, ssdp_shutdown_rx));

    // HTTP server with graceful shutdown.
    let mut http_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move { let _ = http_rx.recv().await; })
            .await
            .unwrap_or_else(|e| tracing::error!("HTTP server error: {}", e));
    });

    // Wait for first Ctrl+C.
    wait_for_shutdown().await;
    tracing::info!("Shutting down — sending SSDP byebye...");

    // Broadcast shutdown to all tasks.
    let _ = shutdown_tx.send(());

    // Wait up to 1 second for SSDP byebye to complete.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), ssdp_task).await;

    tracing::info!("Goodbye.");
}

/// Run dual-stack (IPv4 + IPv6) HTTP + SSDP server and wait for graceful shutdown.
async fn run_dual_stack(
    port: u16,
    server_name: String,
    server_uuid: String,
    library: Arc<RwLock<MediaLibrary>>,
    app: Router,
) {
    // Dual-bind: separate IPv4 (0.0.0.0) and IPv6 (:::) sockets.
    // Use socket2 for IPv6 to explicitly set IPV6_V6ONLY=true.
    // Linux defaults IPV6_V6ONLY=false (shared stack), which causes
    // "Address already in use" when both 0.0.0.0 and ::: are bound.
    // Setting IPV6_V6ONLY=true makes both sockets independent on all OSes.
    let ipv4_addr = format!("0.0.0.0:{}", port);
    let item_count = library.read().expect("library lock poisoned").items.len();
    tracing::info!(
        "Serving {} media items on port {} (IPv4 + IPv6)",
        item_count,
        port
    );

    let ipv4_listener = tokio::net::TcpListener::bind(&ipv4_addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("error: failed to bind IPv4 {}: {}", ipv4_addr, e);
            std::process::exit(1);
        });

    // "[::]:port" is the Rust SocketAddr syntax for the IPv6 any-address (:::port in CLI notation)
    let ipv6_addr_parsed: std::net::SocketAddr = format!("[::]:{}",port)
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("error: failed to parse IPv6 address: {}", e);
            std::process::exit(1);
        });
    let ipv6_raw = socket2::Socket::new(
        socket2::Domain::IPV6,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    ).unwrap_or_else(|e| {
        eprintln!("error: failed to create IPv6 socket: {}", e);
        std::process::exit(1);
    });
    ipv6_raw.set_only_v6(true).unwrap_or_else(|e| {
        tracing::warn!("Could not set IPV6_V6ONLY: {} -- dual-bind may fail on Linux", e);
    });
    ipv6_raw.set_reuse_address(true).unwrap_or_else(|e| {
        tracing::warn!("Could not set SO_REUSEADDR on IPv6 socket: {}", e);
    });
    ipv6_raw.set_nonblocking(true).unwrap_or_else(|e| {
        eprintln!("error: failed to set IPv6 socket non-blocking: {}", e);
        std::process::exit(1);
    });
    ipv6_raw.bind(&ipv6_addr_parsed.into()).unwrap_or_else(|e| {
        eprintln!("error: failed to bind IPv6 :::{}: {}", port, e);
        std::process::exit(1);
    });
    ipv6_raw.listen(1024).unwrap_or_else(|e| {
        eprintln!("error: failed to listen on IPv6 socket: {}", e);
        std::process::exit(1);
    });
    let ipv6_std_listener: std::net::TcpListener = ipv6_raw.into();
    let ipv6_listener = tokio::net::TcpListener::from_std(ipv6_std_listener).unwrap_or_else(|e| {
        eprintln!("error: failed to convert IPv6 listener to tokio: {}", e);
        std::process::exit(1);
    });

    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(4);

    // SSDP task: spawned after both HTTP listeners are bound.
    let ssdp_config = ssdp::service::SsdpConfig {
        device_uuid: server_uuid,
        http_port: port,
        server_name,
    };
    let ssdp_shutdown_rx = shutdown_tx.subscribe();
    let ssdp_task = tokio::spawn(ssdp::service::run(ssdp_config, ssdp_shutdown_rx));

    // HTTP tasks with graceful shutdown.
    let app_v4 = app.clone();
    let mut http_v4_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        axum::serve(ipv4_listener, app_v4)
            .with_graceful_shutdown(async move { let _ = http_v4_rx.recv().await; })
            .await
            .unwrap_or_else(|e| tracing::error!("IPv4 server error: {}", e));
    });
    let mut http_v6_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        axum::serve(ipv6_listener, app)
            .with_graceful_shutdown(async move { let _ = http_v6_rx.recv().await; })
            .await
            .unwrap_or_else(|e| tracing::error!("IPv6 server error: {}", e));
    });

    // Wait for first Ctrl+C.
    wait_for_shutdown().await;
    tracing::info!("Shutting down — sending SSDP byebye...");

    // Broadcast shutdown to all tasks.
    let _ = shutdown_tx.send(());

    // Wait up to 1 second for SSDP byebye to complete.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), ssdp_task).await;

    tracing::info!("Goodbye.");
    // HTTP tasks drain in-flight requests automatically via with_graceful_shutdown.
    // Process exits here; HTTP tasks are killed by process exit.
}
