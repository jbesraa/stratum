pub mod config;
pub mod error;
pub mod job_declarator;
pub mod mempool;
pub mod status;
use async_channel::{bounded, unbounded, Receiver, Sender};
use config::JobDeclaratorServerConfig;
use error::JdsError;
use job_declarator::JobDeclarator;
use mempool::error::JdsMempoolError;
use roles_logic_sv2::utils::Mutex;
pub use rpc_sv2::Uri;
use std::{ops::Sub, str::FromStr, sync::Arc};
use tokio::{select, task};
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct JobDeclaratorServer {
    config: JobDeclaratorServerConfig,
    status_tx: Arc<std::sync::Mutex<Option<async_channel::Sender<status::Status>>>>,
}

impl JobDeclaratorServer {
    pub fn new(config: JobDeclaratorServerConfig) -> Self {
        Self {
            config,
            status_tx: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub async fn start(&self) -> Result<&Self, JdsError> {
        let mut config = self.config.clone();
        // In case the url came with a trailing slash, we remove it to make sure we end up with
        // `{scheme}://{host}:{port}` format.
        config.set_core_rpc_url(config.core_rpc_url().trim_end_matches('/').to_string());
        let url = config.core_rpc_url().to_string() + ":" + &config.core_rpc_port().to_string();
        let username = config.core_rpc_user();
        let password = config.core_rpc_pass();
        // TODO should we manage what to do when the limit is reaced?
        let (new_block_sender, new_block_receiver): (Sender<String>, Receiver<String>) =
            bounded(10);
        let url = Uri::from_str(&url.clone()).expect("Invalid core rpc url");
        let mempool = Arc::new(Mutex::new(mempool::JDsMempool::new(
            url,
            username.to_string(),
            password.to_string(),
            new_block_receiver,
        )));
        let mempool_update_interval = config.mempool_update_interval();
        let mempool_cloned_ = mempool.clone();
        let mempool_cloned_1 = mempool.clone();
        if let Err(e) = mempool::JDsMempool::health(mempool_cloned_1.clone()).await {
            error!("JDS Connection with bitcoin core failed {:?}", e);
            return Err(JdsError::MempoolError(e));
        }
        let (status_tx, status_rx) = unbounded();
        if let Ok(mut s_tx) = self.status_tx.lock() {
            *s_tx = Some(status_tx.clone());
        } else {
            error!("Failed to access JDS status lock");
            return Err(JdsError::Custom(
                "Failed to access JDS status lock".to_string(),
            ));
        }
        let (send_stop_signal, mut recv_stop_signal) = tokio::sync::watch::channel(());
        let mut cloned_recv_stop_signal_1 = recv_stop_signal.clone();
        let mut cloned_recv_stop_signal_2 = recv_stop_signal.clone();
        let mut cloned_recv_stop_signal_3 = recv_stop_signal.clone();
        let sender = status::Sender::Downstream(status_tx.clone());
        let mut last_empty_mempool_warning =
            std::time::Instant::now().sub(std::time::Duration::from_secs(60));
        task::spawn(async move {
            select!(
            _ = recv_stop_signal.changed() => {
                info!("Job Declarator Server: Received stop signal, shutting down.");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Update Mempool(JDS): Received ctrl-c signal, shutting down.");
            }
            _ = async {
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
                                error!("{:?}", err);
                                error!("Unable to establish RPC connection with Template Provider (possible reasons: not fully synced, down)");
                            }
                            JdsMempoolError::Rpc(_) => {
                                mempool::error::handle_error(&err);
                                error!("{:?}", err);
                                error!("Unable to establish RPC connection with Template Provider (possible reasons: not fully synced, down)");
                            }
                            JdsMempoolError::PoisonLock(_) => {
                                mempool::error::handle_error(&err);
                                error!("{:?}", err);
                                error!("Poison lock error)");
                            }
                        }
                    }
                    tokio::time::sleep(mempool_update_interval).await;
                }
            } => {}
            )
        });

        let mempool_cloned = mempool.clone();
        task::spawn(async move {
            loop {
                select!(
                _ = cloned_recv_stop_signal_1.changed() => {
                    break info!("OnSubmit(JDS): Received stop signal, shutting down.");
                }
                _ = tokio::signal::ctrl_c() => {
                    break info!("OnSubmit(JDS): Received ctrl-c signal, shutting down.");
                }
                _ = async {
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
                                // handle_result!(sender_submit_solution, Err(err));
                            }
                        }
                    }
                } => {}

                );
            }
        });

        let cloned = config.clone();
        let mempool_cloned = mempool.clone();
        let (sender_add_txs_to_mempool, receiver_add_txs_to_mempool) = unbounded();

        task::spawn(async move {
            select!(
                _ = cloned_recv_stop_signal_2.changed() => {
                    info!("Job Declarator Server: Received stop signal, shutting down.");
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Job Declarator Server: Received ctrl-c signal, shutting down.");
                }
                _ = async {
                    JobDeclarator::start(
                        cloned,
                        sender,
                        mempool_cloned,
                        new_block_sender,
                        sender_add_txs_to_mempool,
                    )
                        .await
                } => {}
            );
        });

        task::spawn(async move {
            loop {
                select!(
                _ = cloned_recv_stop_signal_3.changed() => {
                    info!("Add TX data to mempool(JDS): Received stop signal, shutting down.");
                }
                _ = tokio::signal::ctrl_c() => {
                    break info!("Add TX data to mempool(JDS): Received ctrl-c signal, shutting down.");
                }
                _ = async {
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
                } => {}
                );
            }
        });

        tokio::spawn(async move {
            loop {
                let task_status = select! {
                    task_status = status_rx.recv() => task_status.unwrap(),
                    _ = tokio::signal::ctrl_c() => {
                        break info!("Job Declarator Server State Manager: Received ctrl-c signal, shutting down.");
                    }
                };

                match task_status.state {
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
                    status::State::Shutdown => {
                        info!("Job Declarator Server State Manager: Received stop signal, shutting down.");
                        let _ = send_stop_signal.send(());
                        break;
                    }
                }
            }
        });
        Ok(self)
    }

    /// Shutdown Job Declarator Server and all associated tasks
    pub fn shutdown(&self) {
        info!("Attempting to shutdown JDS");
        if let Ok(status_tx) = &self.status_tx.lock() {
            if let Some(status_tx) = status_tx.as_ref().cloned() {
                info!("JDS is running, sending shutdown signal");
                tokio::spawn(async move {
                    if let Err(e) = status_tx
                        .send(status::Status {
                            state: status::State::Shutdown,
                        })
                        .await
                    {
                        error!("Failed to send shutdown signal to status loop: {:?}", e);
                    } else {
                        info!("Sent shutdown signal to JDS");
                    }
                });
            } else {
                info!("JDS is not running.");
            }
        } else {
            error!("Failed to access JDS status lock");
        }
    }
}

#[cfg(test)]
mod tests {}
