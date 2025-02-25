#![allow(dead_code)]
use key_utils::{Secp256k1PublicKey, Secp256k1SecretKey};
use roles_logic_sv2::{errors::Error, utils::CoinbaseOutput as CoinbaseOutput_};
use serde::Deserialize;
use std::{net::SocketAddr, time::Duration};
use stratum_common::{bitcoin::TxOut, coinbase_output::CoinbaseOutput};

/// Represents the configuration of a job declarator client.
///
/// Job Declarator Client can act as both, an upstream and a downstream.
///
/// Usually when JDC acts as a downstream, it connects to a Pool. An this data can be provided to
/// the [`JobDeclaratorClientConfig::upstreams`] field. Note that this is a vector of upstreams as
/// JDC will try to fallback to the next upstream in case of failure of the current one.
///
/// JDC also connects to a Template Provider (TP) to get the templates. The TP address can be
/// provided in the [`JobDeclaratorClientConfig::tp_address`] field.
#[derive(Debug, Deserialize, Clone)]
pub struct JobDeclaratorClientConfig {
    listening_address: SocketAddr,
    max_supported_version: u16,
    min_supported_version: u16,
    min_extranonce2_size: u16,
    withhold: bool,
    authority_public_key: Secp256k1PublicKey,
    authority_secret_key: Secp256k1SecretKey,
    cert_validity_sec: u64,
    tp_address: String,
    tp_authority_public_key: Option<Secp256k1PublicKey>,
    upstreams: Vec<Upstream>,
    #[serde(deserialize_with = "stratum_common::toml::duration_from_toml")]
    timeout: Duration,
    coinbase_outputs: Vec<CoinbaseOutput>,
}

impl JobDeclaratorClientConfig {
    /// Creates a new instance of [`JobDeclaratorClientConfig`].
    pub fn new(
        listening_address: SocketAddr,
        protocol_config: ProtocolConfig,
        withhold: bool,
        pool_config: PoolConfig,
        tp_config: TPConfig,
        upstreams: Vec<Upstream>,
        timeout: Duration,
    ) -> Self {
        Self {
            listening_address,
            max_supported_version: protocol_config.max_supported_version,
            min_supported_version: protocol_config.min_supported_version,
            min_extranonce2_size: protocol_config.min_extranonce2_size,
            withhold,
            authority_public_key: pool_config.authority_public_key,
            authority_secret_key: pool_config.authority_secret_key,
            cert_validity_sec: tp_config.cert_validity_sec,
            tp_address: tp_config.tp_address,
            tp_authority_public_key: tp_config.tp_authority_public_key,
            upstreams,
            timeout,
            coinbase_outputs: protocol_config.coinbase_outputs,
        }
    }

    /// Returns the listening address of the Job Declartor Client.
    pub fn listening_address(&self) -> &SocketAddr {
        &self.listening_address
    }

    /// Returns "Minimum extranonce2" size.
    pub fn min_extranonce2_size(&self) -> u16 {
        self.min_extranonce2_size
    }

    /// Returns the list of upstreams.
    ///
    /// JDC will try to fallback to the next upstream in case of failure of the current one.
    pub fn upstreams(&self) -> &Vec<Upstream> {
        &self.upstreams
    }

    /// Returns the timeout duration.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Returns the withhold flag.
    pub fn withhold(&self) -> bool {
        self.withhold
    }

    /// Returns the authority public key.
    pub fn authority_public_key(&self) -> &Secp256k1PublicKey {
        &self.authority_public_key
    }

    /// Returns the authority secret key.
    pub fn authority_secret_key(&self) -> &Secp256k1SecretKey {
        &self.authority_secret_key
    }

    /// Returns the certificate validity in seconds.
    pub fn cert_validity_sec(&self) -> u64 {
        self.cert_validity_sec
    }

    /// Returns Template Provider address.
    pub fn tp_address(&self) -> &str {
        &self.tp_address
    }

    /// Returns Template Provider authority public key.
    pub fn tp_authority_public_key(&self) -> Option<&Secp256k1PublicKey> {
        self.tp_authority_public_key.as_ref()
    }

    /// Returns the minimum supported version.
    pub fn min_supported_version(&self) -> u16 {
        self.min_supported_version
    }

    /// Returns the maximum supported version.
    pub fn max_supported_version(&self) -> u16 {
        self.max_supported_version
    }
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
