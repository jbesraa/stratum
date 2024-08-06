#![allow(special_module_name)]
mod args;
mod lib;

use async_channel::unbounded;
use futures::{select, FutureExt};
pub use roles_logic_sv2::{
    mining_sv2::{ExtendedExtranonce, NewExtendedMiningJob, SetNewPrevHash, SubmitSharesExtended},
    utils::Mutex,
};
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    thread::sleep,
    time::Duration,
};

use tracing::{error, info};
pub use v1::server_to_client;

use args::Args;
use error::{Error, ProxyResult};
use lib::{downstream_sv1, error, proxy, proxy_config, status, upstream_sv2};
use proxy_config::ProxyConfig;

use ext_config::{Config, File, FileFormat};

/// Process CLI args, if any.
#[allow(clippy::result_large_err)]
fn process_cli_args<'a>() -> ProxyResult<'a, ProxyConfig> {
    // Parse CLI arguments
    let args = Args::from_args().map_err(|help| {
        error!("{}", help);
        Error::BadCliArgs
    })?;

    // Build configuration from the provided file path
    let config_path = args.config_path.to_str().ok_or_else(|| {
        error!("Invalid configuration path.");
        Error::BadCliArgs
    })?;

    let settings = Config::builder()
        .add_source(File::new(config_path, FileFormat::Toml))
        .build()?;

    // Deserialize settings into ProxyConfig
    let config = settings.try_deserialize::<ProxyConfig>()?;
    Ok(config)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = match process_cli_args() {
        Ok(p) => p,
        Err(e) => panic!("failed to load config: {}", e),
    };
    info!("PC: {:?}", &config);

    // Connect to the SV2 Upstream role retry connection every 5 seconds.
    let upstream_address = config.upstream_address.clone();
    let upstream_address = SocketAddr::new(
        IpAddr::from_str(&upstream_address).unwrap(),
        config.upstream_port,
    );
    let upstream_connection = loop {
        match async_std::net::TcpStream::connect(upstream_address).await {
            Ok(socket) => break socket,
            Err(e) => {
                error!(
                    "Failed to connect to Upstream role at {}, retrying in 5s: {}",
                    upstream_address, e
                );
                sleep(Duration::from_secs(5));
            }
        }
    };

    let translator_sv2 = lib::TranslatorSv2::new(upstream_connection);
    translator_sv2.start(config).await;
}
