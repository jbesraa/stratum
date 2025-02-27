pub mod config;
pub mod error;
pub mod job_declarator;
pub mod mempool;
pub mod status;
use async_channel::{bounded, unbounded, Receiver, Sender};
use codec_sv2::{StandardEitherFrame, StandardSv2Frame};
use config::JobDeclaratorServerConfig;
use error::JdsError;
use error_handling::handle_result;
use job_declarator::JobDeclarator;
use mempool::error::JdsMempoolError;
use roles_logic_sv2::{parsers::PoolMessages as JdsMessages, utils::Mutex};
use std::{ops::Sub, sync::Arc};
use stratum_common::url::is_valid_url;
use tokio::{select, task};
use tracing::{error, info, warn};

pub type Message = JdsMessages<'static>;
pub type StdFrame = StandardSv2Frame<Message>;
pub type EitherFrame = StandardEitherFrame<Message>;

#[derive(Debug, Clone)]
pub struct JobDeclaratorServer {
    config: JobDeclaratorServerConfig,
}

impl JobDeclaratorServer {
    pub fn new(config: JobDeclaratorServerConfig) -> Result<Self, Box<JdsError>> {
        let url = config.core_rpc_url().to_string() + ":" + &config.core_rpc_port().to_string();
        if !is_valid_url(&url) {
            return Err(Box::new(JdsError::InvalidRPCUrl));
        }
        Ok(Self { config })
    }
    pub async fn start(&self) -> Result<(), JdsError> {
        let config = self.config.clone();
        let url = config.core_rpc_url().to_string() + ":" + &config.core_rpc_port().to_string();
        let username = config.core_rpc_user();
        let password = config.core_rpc_pass();
        // TODO should we manage what to do when the limit is reaced?
        let (new_block_sender, new_block_receiver): (Sender<String>, Receiver<String>) =
            bounded(10);
        let mempool = Arc::new(Mutex::new(mempool::JDsMempool::new(
            url.clone(),
            username.to_string(),
            password.to_string(),
            new_block_receiver,
        )));
        let mempool_update_interval = config.mempool_update_interval();
        let mempool_cloned_ = mempool.clone();
        let mempool_cloned_1 = mempool.clone();
        if let Err(e) = mempool::JDsMempool::health(mempool_cloned_1.clone()).await {
            error!("{:?}", e);
            return Err(JdsError::MempoolError(e));
        }
        let (status_tx, status_rx) = unbounded();
        let sender = status::Sender::Downstream(status_tx.clone());
        let mut last_empty_mempool_warning =
            std::time::Instant::now().sub(std::time::Duration::from_secs(60));

        let sender_update_mempool = sender.clone();
        task::spawn(async move {
            loop {
                let update_mempool_result: Result<(), mempool::error::JdsMempoolError> =
                    mempool::JDsMempool::update_mempool(mempool_cloned_.clone()).await;
                if let Err(err) = update_mempool_result {
                    match err {
                        JdsMempoolError::EmptyMempool => {
                            if last_empty_mempool_warning.elapsed().as_secs() >= 60 {
                                warn!("{:?}", err);
                                warn!("Template Provider is running, but its mempool is empty (possible reasons: you're testing in testnet, signet, or regtest)");
                                last_empty_mempool_warning = std::time::Instant::now();
                            }
                        }
                        JdsMempoolError::NoClient => {
                            mempool::error::handle_error(&err);
                            handle_result!(sender_update_mempool, Err(err));
                        }
                        JdsMempoolError::Rpc(_) => {
                            mempool::error::handle_error(&err);
                            handle_result!(sender_update_mempool, Err(err));
                        }
                        JdsMempoolError::PoisonLock(_) => {
                            mempool::error::handle_error(&err);
                            handle_result!(sender_update_mempool, Err(err));
                        }
                    }
                }
                tokio::time::sleep(mempool_update_interval).await;
                // DO NOT REMOVE THIS LINE
                //let _transactions =
                // mempool::JDsMempool::_get_transaction_list(mempool_cloned_.clone());
            }
        });

        let mempool_cloned = mempool.clone();
        let sender_submit_solution = sender.clone();
        task::spawn(async move {
            loop {
                let result = mempool::JDsMempool::on_submit(mempool_cloned.clone()).await;
                if let Err(err) = result {
                    match err {
                        JdsMempoolError::EmptyMempool => {
                            if last_empty_mempool_warning.elapsed().as_secs() >= 60 {
                                warn!("{:?}", err);
                                warn!("Template Provider is running, but its mempool is empty (possible reasons: you're testing in testnet, signet, or regtest)");
                                last_empty_mempool_warning = std::time::Instant::now();
                            }
                        }
                        _ => {
                            // TODO here there should be a better error managmenet
                            mempool::error::handle_error(&err);
                            handle_result!(sender_submit_solution, Err(err));
                        }
                    }
                }
            }
        });

        let cloned = config.clone();
        let mempool_cloned = mempool.clone();
        let (sender_add_txs_to_mempool, receiver_add_txs_to_mempool) = unbounded();
        task::spawn(async move {
            JobDeclarator::start(
                cloned,
                sender,
                mempool_cloned,
                new_block_sender,
                sender_add_txs_to_mempool,
            )
            .await
        });
        task::spawn(async move {
            loop {
                if let Ok(add_transactions_to_mempool) = receiver_add_txs_to_mempool.recv().await {
                    let mempool_cloned = mempool.clone();
                    task::spawn(async move {
                        match mempool::JDsMempool::add_tx_data_to_mempool(
                            mempool_cloned,
                            add_transactions_to_mempool,
                        )
                        .await
                        {
                            Ok(_) => (),
                            Err(err) => {
                                // TODO
                                // here there should be a better error management
                                mempool::error::handle_error(&err);
                            }
                        }
                    });
                }
            }
        });

        // Start the error handling loop
        // See `./status.rs` and `utils/error_handling` for information on how this operates
        loop {
            let task_status = select! {
                task_status = status_rx.recv() => task_status,
                interrupt_signal = tokio::signal::ctrl_c() => {
                    match interrupt_signal {
                        Ok(()) => {
                            info!("Interrupt received");
                        },
                        Err(err) => {
                            error!("Unable to listen for interrupt signal: {}", err);
                            // we also shut down in case of error
                        },
                    }
                    break;
                }
            };
            let task_status: status::Status = task_status.unwrap();

            match task_status.state {
                // Should only be sent by the downstream listener
                status::State::DownstreamShutdown(err) => {
                    error!(
                        "SHUTDOWN from Downstream: {}\nTry to restart the downstream listener",
                        err
                    );
                }
                status::State::TemplateProviderShutdown(err) => {
                    error!("SHUTDOWN from Upstream: {}\nTry to reconnecting or connecting to a new upstream", err);
                    break;
                }
                status::State::Healthy(msg) => {
                    info!("HEALTHY message: {}", msg);
                }
                status::State::DownstreamInstanceDropped(downstream_id) => {
                    warn!("Dropping downstream instance {} from jds", downstream_id);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ext_config::{Config, File, FileFormat};
    use roles_logic_sv2::utils::CoinbaseOutput as CoinbaseOutput_;
    use std::{convert::TryInto, path::PathBuf};
    use stratum_common::bitcoin::{Amount, ScriptBuf, TxOut};

    use crate::config::JobDeclaratorServerConfig;

    fn load_config(path: &str) -> JobDeclaratorServerConfig {
        let config_path = PathBuf::from(path);
        assert!(
            config_path.exists(),
            "No config file found at {:?}",
            config_path
        );

        let config_path = config_path.to_str().unwrap();

        let settings = Config::builder()
            .add_source(File::new(&config_path, FileFormat::Toml))
            .build()
            .expect("Failed to build config");

        settings.try_deserialize().expect("Failed to parse config")
    }

    #[tokio::test]
    async fn test_invalid_rpc_url() {
        let mut config = load_config("config-examples/jds-config-hosted-example.toml");
        config.set_core_rpc_url("invalid".to_string());
        assert!(super::JobDeclaratorServer::new(config).is_err());
    }

    #[tokio::test]
    async fn test_offline_rpc_url() {
        let mut config = load_config("config-examples/jds-config-hosted-example.toml");
        config.set_core_rpc_url("http://127.0.0.1".to_string());
        let jd = super::JobDeclaratorServer::new(config).unwrap();
        assert!(jd.start().await.is_err());
    }

    #[test]
    fn test_get_coinbase_output_non_empty() {
        let config = load_config("config-examples/jds-config-hosted-example.toml");
        let outputs = config.get_txout().expect("Failed to get coinbase output");

        let expected_output = CoinbaseOutput_ {
            output_script_type: "P2WPKH".to_string(),
            output_script_value:
                "036adc3bdf21e6f9a0f0fb0066bf517e5b7909ed1563d6958a10993849a7554075".to_string(),
        };
        let expected_script: ScriptBuf = expected_output.try_into().unwrap();
        let expected_transaction_output = TxOut {
            value: Amount::from_sat(0),
            script_pubkey: expected_script,
        };

        assert_eq!(outputs[0], expected_transaction_output);
    }

    #[test]
    fn test_get_coinbase_output_empty() {
        let mut config = load_config("config-examples/jds-config-hosted-example.toml");
        config.set_coinbase_outputs(Vec::new());

        let result = &config.get_txout();
        assert!(
            matches!(result, Err(roles_logic_sv2::Error::EmptyCoinbaseOutputs)),
            "Expected an error for empty coinbase outputs"
        );
    }

    #[test]
    fn test_try_from_valid_input() {
        let input = config_helpers::CoinbaseOutput::new(
            "P2PKH".to_string(),
            "036adc3bdf21e6f9a0f0fb0066bf517e5b7909ed1563d6958a10993849a7554075".to_string(),
        );
        let result: Result<CoinbaseOutput_, _> = (&input).try_into();
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_invalid_input() {
        let input = config_helpers::CoinbaseOutput::new(
            "INVALID".to_string(),
            "036adc3bdf21e6f9a0f0fb0066bf517e5b7909ed1563d6958a10993849a7554075".to_string(),
        );
        let result: Result<CoinbaseOutput_, _> = (&input).try_into();
        assert!(matches!(
            result,
            Err(roles_logic_sv2::Error::UnknownOutputScriptType)
        ));
    }
}
