use aero_relay::{config::Config, ibc::IbcPoller, transport};
use anyhow::Result;
use std::time::Duration;
use tracing::{error, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, layer::SubscriberExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize crypto provider for QUIC (aws-lc-rs)
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install crypto provider");

    // Log directory (relative to project root)
    let log_path = "logs";

    // Create logs directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(log_path) {
        eprintln!("Failed to create logs directory: {}. Logs will go to console only.", e);
    }

    // Daily rotating file appender
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_path, "aero-relay.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer()) // Pretty console output
        .with(fmt::layer().with_writer(non_blocking_file)) // File output
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")), // Default to info if RUST_LOG unset
        )
        .init();

    info!("AeroRelay starting... ✈️");

    let config = Config::load("config.toml")?;

    // Start QUIC server once (in background)
    tokio::spawn(async move {
        info!("QUIC Server listening on 0.0.0.0:4433");
        if let Err(e) = transport::start_server("0.0.0.0:4433").await {
            error!("QUIC Server error: {}", e);
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    for relay in config.relays {
        info!("Setting up relay: {}", relay.name);

        let src_rpc = relay.src_rpc.clone();
        let src_channel = relay.src_channel.clone();

        // Spawn poller for each relay
        tokio::spawn(async move {
            match IbcPoller::new(&src_rpc, &src_channel).await {
                Ok(mut poller) => {
                    if let Err(e) = poller.poll().await {
                        error!("Polling error [{}]: {}", src_channel, e);
                    }
                }
                Err(e) => error!("Failed to initialize poller [{}]: {}", src_channel, e),
            }
        });
    }

    info!("AeroRelay fully started. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    Ok(())
}