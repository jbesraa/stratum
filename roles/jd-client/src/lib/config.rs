#![allow(dead_code)]
use key_utils::{Secp256k1PublicKey, Secp256k1SecretKey};
use roles_logic_sv2::{errors::Error, utils::CoinbaseOutput as CoinbaseOutput_};
use serde::Deserialize;
use std::time::Duration;
use stratum_common::{bitcoin::TxOut, coinbase_output::CoinbaseOutput};

#[derive(Debug, Deserialize, Clone)]
pub struct JobDeclaratorClientConfig {
    pub downstream_address: String,
    pub downstream_port: u16,
    pub max_supported_version: u16,
    pub min_supported_version: u16,
    pub min_extranonce2_size: u16,
    pub withhold: bool,
    pub authority_public_key: Secp256k1PublicKey,
    pub authority_secret_key: Secp256k1SecretKey,
    pub cert_validity_sec: u64,
    pub tp_address: String,
    pub tp_authority_public_key: Option<Secp256k1PublicKey>,
    #[allow(dead_code)]
    pub retry: u32,
    pub upstreams: Vec<Upstream>,
    #[serde(deserialize_with = "stratum_common::toml::duration_from_toml")]
    pub timeout: Duration,
    pub coinbase_outputs: Vec<CoinbaseOutput>,
    pub test_only_do_not_send_solution_to_tp: Option<bool>,
}

pub struct PoolConfig {
    authority_public_key: Secp256k1PublicKey,
    authority_secret_key: Secp256k1SecretKey,
}

impl PoolConfig {
    pub fn new(
        authority_public_key: Secp256k1PublicKey,
        authority_secret_key: Secp256k1SecretKey,
    ) -> Self {
        Self {
            authority_public_key,
            authority_secret_key,
        }
    }
}

pub struct TPConfig {
    cert_validity_sec: u64,
    tp_address: String,
    tp_authority_public_key: Option<Secp256k1PublicKey>,
}

impl TPConfig {
    pub fn new(
        cert_validity_sec: u64,
        tp_address: String,
        tp_authority_public_key: Option<Secp256k1PublicKey>,
    ) -> Self {
        Self {
            cert_validity_sec,
            tp_address,
            tp_authority_public_key,
        }
    }
}

pub struct ProtocolConfig {
    max_supported_version: u16,
    min_supported_version: u16,
    min_extranonce2_size: u16,
    coinbase_outputs: Vec<CoinbaseOutput>,
}

impl ProtocolConfig {
    pub fn new(
        max_supported_version: u16,
        min_supported_version: u16,
        min_extranonce2_size: u16,
        coinbase_outputs: Vec<CoinbaseOutput>,
    ) -> Self {
        Self {
            max_supported_version,
            min_supported_version,
            min_extranonce2_size,
            coinbase_outputs,
        }
    }
}

impl JobDeclaratorClientConfig {
    pub fn new(
        listening_address: std::net::SocketAddr,
        protocol_config: ProtocolConfig,
        withhold: bool,
        pool_config: PoolConfig,
        tp_config: TPConfig,
        upstreams: Vec<Upstream>,
        timeout: Duration,
    ) -> Self {
        Self {
            downstream_address: listening_address.ip().to_string(),
            downstream_port: listening_address.port(),
            max_supported_version: protocol_config.max_supported_version,
            min_supported_version: protocol_config.min_supported_version,
            min_extranonce2_size: protocol_config.min_extranonce2_size,
            withhold,
            authority_public_key: pool_config.authority_public_key,
            authority_secret_key: pool_config.authority_secret_key,
            cert_validity_sec: tp_config.cert_validity_sec,
            tp_address: tp_config.tp_address,
            tp_authority_public_key: tp_config.tp_authority_public_key,
            retry: 0,
            upstreams,
            timeout,
            coinbase_outputs: protocol_config.coinbase_outputs,
            test_only_do_not_send_solution_to_tp: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Upstream {
    pub authority_pubkey: Secp256k1PublicKey,
    pub pool_address: String,
    pub jd_address: String,
    pub pool_signature: String, // string be included in coinbase tx input scriptsig
}

impl Upstream {
    pub fn new(
        authority_pubkey: Secp256k1PublicKey,
        pool_address: String,
        jd_address: String,
        pool_signature: String,
    ) -> Self {
        Self {
            authority_pubkey,
            pool_address,
            jd_address,
            pool_signature,
        }
    }
}

pub fn get_coinbase_output(config: &JobDeclaratorClientConfig) -> Result<Vec<TxOut>, Error> {
    let mut result = Vec::new();
    for coinbase_output_pool in &config.coinbase_outputs {
        let coinbase_output: CoinbaseOutput_ = coinbase_output_pool.try_into()?;
        let output_script = coinbase_output.try_into()?;
        result.push(TxOut {
            value: 0,
            script_pubkey: output_script,
        });
    }
    match result.is_empty() {
        true => Err(Error::EmptyCoinbaseOutputs),
        _ => Ok(result),
    }
}
