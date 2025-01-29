pub mod error;
pub mod mining_pool;
pub mod status;
pub mod template_receiver;

use std::net::SocketAddr;

use async_channel::{bounded, unbounded};

use error::PoolError;
use mining_pool::{get_coinbase_output, Configuration, Pool};
use template_receiver::TemplateRx;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct PoolSv2 {
    config: Configuration,
}

impl PoolSv2 {
    pub fn new(config: Configuration) -> PoolSv2 {
        PoolSv2 { config }
    }

    pub fn start(&self) -> Result<(), PoolError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let config = self.config.clone();
        let (status_tx, status_rx) = unbounded();
        let (s_new_t, r_new_t) = bounded(10);
        let (s_prev_hash, r_prev_hash) = bounded(10);
        let (s_solution, r_solution) = bounded(10);
        let (s_message_recv_signal, r_message_recv_signal) = bounded(10);
        let coinbase_output_result = get_coinbase_output(&config);
        let coinbase_output_len = coinbase_output_result?.len() as u32;
        let tp_authority_public_key = config.tp_authority_public_key;
        let tp_address = config.tp_address.parse::<SocketAddr>().unwrap().clone();
        let clone_status_tx = status_tx.clone();
        runtime.spawn(async move {
            TemplateRx::connect(
                tp_address,
                s_new_t,
                s_prev_hash,
                r_solution,
                r_message_recv_signal,
                status::Sender::Upstream(clone_status_tx),
                coinbase_output_len,
                tp_authority_public_key,
            )
            .await
            .unwrap();
        });
        let pool = Pool::start(
            config.clone(),
            r_new_t,
            r_prev_hash,
            s_solution,
            s_message_recv_signal,
            status::Sender::DownstreamListener(status_tx),
        );

        // Start the error handling loop
        // See `./status.rs` and `utils/error_handling` for information on how this operates

        let status_loop = move || {
            runtime.spawn(async move {
                loop {
                    let task_status = status_rx.recv().await;
                    let task_status: status::Status = task_status.unwrap();

                    match task_status.state {
                        // Should only be sent by the downstream listener
                        status::State::DownstreamShutdown(err) => {
                            error!(
                                "SHUTDOWN from Downstream: {}\nTry to restart the downstream listener",
                                err
                            );
                            break;
                        }
                        status::State::TemplateProviderShutdown(err) => {
                            error!("SHUTDOWN from Upstream: {}\nTry to reconnecting or connecting to a new upstream", err);
                            break;
                        }
                        status::State::Healthy(msg) => {
                            info!("HEALTHY message: {}", msg);
                        }
                        status::State::DownstreamInstanceDropped(downstream_id) => {
                            warn!("Dropping downstream instance {} from pool", downstream_id);
                            if pool
                                .safe_lock(|p| p.remove_downstream(downstream_id))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
            })
        };
        status_loop();
        // std::thread::spawn(status_loop);
        // status_loop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ext_config::{Config, File, FileFormat};

    #[tokio::test]
    async fn pool_bad_coinbase_output() {
        let invalid_coinbase_output = vec![mining_pool::CoinbaseOutput::new(
            "P2PK".to_string(),
            "wrong".to_string(),
        )];
        let config_path = "config-examples/pool-config-hosted-tp-example.toml";
        let mut config: Configuration = match Config::builder()
            .add_source(File::new(config_path, FileFormat::Toml))
            .build()
        {
            Ok(settings) => match settings.try_deserialize::<Configuration>() {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to deserialize config: {}", e);
                    return;
                }
            },
            Err(e) => {
                error!("Failed to build config: {}", e);
                return;
            }
        };
        config.coinbase_outputs = invalid_coinbase_output;
        let pool = PoolSv2::new(config);
        let result = pool.start().await;
        assert!(result.is_err());
    }
}
