use key_utils::{Secp256k1PublicKey, Secp256k1SecretKey};
use serde::Deserialize;
use std::time::Duration;
use stratum_common::coinbase_output::CoinbaseOutput;

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

    pub fn listen_jd_address(&self) -> &str {
        &self.listen_jd_address
    }

    pub fn authority_public_key(&self) -> &Secp256k1PublicKey {
        &self.authority_public_key
    }

    pub fn authority_secret_key(&self) -> &Secp256k1SecretKey {
        &self.authority_secret_key
    }

    pub fn core_rpc_url(&self) -> &str {
        &self.core_rpc_url
    }

    pub fn core_rpc_port(&self) -> u16 {
        self.core_rpc_port
    }

    pub fn core_rpc_user(&self) -> &str {
        &self.core_rpc_user
    }

    pub fn core_rpc_pass(&self) -> &str {
        &self.core_rpc_pass
    }

    pub fn coinbase_outputs(&self) -> &Vec<CoinbaseOutput> {
        &self.coinbase_outputs
    }

    pub fn cert_validity_sec(&self) -> u64 {
        self.cert_validity_sec
    }

    pub fn async_mining_allowed(&self) -> bool {
        self.async_mining_allowed
    }

    pub fn mempool_update_interval(&self) -> Duration {
        self.mempool_update_interval
    }

    pub fn set_core_rpc_url(&mut self, url: String) {
        self.core_rpc_url = url;
    }

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
