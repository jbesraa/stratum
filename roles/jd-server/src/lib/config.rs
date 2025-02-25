use key_utils::{Secp256k1PublicKey, Secp256k1SecretKey};
use serde::Deserialize;
use std::time::Duration;
use stratum_common::coinbase_output::CoinbaseOutput;

/// Represents the configuration of a Job Declarator Server.
///
/// Job Declarator Server connects through RPC to a Bitcoin core node to get the mempool updates.
/// And it also listens for the connections from the Job Declarator Clients.
#[derive(Debug, serde::Deserialize, Clone)]
pub struct JobDeclaratorServerConfig {
    #[serde(default = "default_true")]
    async_mining_allowed: bool,
    listen_jd_address: String,
    authority_public_key: Secp256k1PublicKey,
    authority_secret_key: Secp256k1SecretKey,
    cert_validity_sec: u64,
    coinbase_outputs: Vec<CoinbaseOutput>,
    core_rpc_url: String,
    core_rpc_port: u16,
    core_rpc_user: String,
    core_rpc_pass: String,
    #[serde(deserialize_with = "stratum_common::toml::duration_from_toml")]
    mempool_update_interval: Duration,
}

impl JobDeclaratorServerConfig {
    /// Creates a new instance of [`JobDeclaratorServerConfig`].
    pub fn new(
        listen_jd_address: String,
        authority_public_key: Secp256k1PublicKey,
        authority_secret_key: Secp256k1SecretKey,
        cert_validity_sec: u64,
        coinbase_outputs: Vec<CoinbaseOutput>,
        core_rpc: CoreRpc,
        mempool_update_interval: Duration,
    ) -> Self {
        Self {
            async_mining_allowed: true,
            listen_jd_address,
            authority_public_key,
            authority_secret_key,
            cert_validity_sec,
            coinbase_outputs,
            core_rpc_url: core_rpc.url,
            core_rpc_port: core_rpc.port,
            core_rpc_user: core_rpc.user,
            core_rpc_pass: core_rpc.pass,
            mempool_update_interval,
        }
    }

    /// Returns the listening address of the Job Declarator Server.
    pub fn listen_jd_address(&self) -> &str {
        &self.listen_jd_address
    }

    /// Returns the public key of the authority.
    pub fn authority_public_key(&self) -> &Secp256k1PublicKey {
        &self.authority_public_key
    }

    /// Returns the secret key of the authority.
    pub fn authority_secret_key(&self) -> &Secp256k1SecretKey {
        &self.authority_secret_key
    }

    /// Returns the URL of the core RPC.
    pub fn core_rpc_url(&self) -> &str {
        &self.core_rpc_url
    }

    /// Returns the port of the core RPC.
    pub fn core_rpc_port(&self) -> u16 {
        self.core_rpc_port
    }

    /// Returns the user of the core RPC.
    pub fn core_rpc_user(&self) -> &str {
        &self.core_rpc_user
    }

    /// Returns the password of the core RPC.
    pub fn core_rpc_pass(&self) -> &str {
        &self.core_rpc_pass
    }

    /// Returns the coinbase outputs.
    pub fn coinbase_outputs(&self) -> &Vec<CoinbaseOutput> {
        &self.coinbase_outputs
    }

    /// Returns the certificate validity in seconds.
    pub fn cert_validity_sec(&self) -> u64 {
        self.cert_validity_sec
    }

    /// Returns whether async mining is allowed.
    pub fn async_mining_allowed(&self) -> bool {
        self.async_mining_allowed
    }

    /// Returns the mempool update interval.
    pub fn mempool_update_interval(&self) -> Duration {
        self.mempool_update_interval
    }

    /// Sets the listening address of Bitcoin core RPC.
    pub fn set_core_rpc_url(&mut self, url: String) {
        self.core_rpc_url = url;
    }

    /// Sets coinbase outputs.
    pub fn set_coinbase_outputs(&mut self, outputs: Vec<CoinbaseOutput>) {
        self.coinbase_outputs = outputs;
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone)]
pub struct CoreRpc {
    url: String,
    port: u16,
    user: String,
    pass: String,
}

impl CoreRpc {
    pub fn new(url: String, port: u16, user: String, pass: String) -> Self {
        Self {
            url,
            port,
            user,
            pass,
        }
    }
}
