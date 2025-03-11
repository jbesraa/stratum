use std::{convert::TryInto, str::FromStr};
use stratum_common::bitcoin::{psbt::OutputType, Amount, ScriptBuf, TxOut};
use tracing::error;

pub enum Error {
    InvalidCoinbaseOutput,
}

#[derive(Debug, Clone)]
struct InternalOutputType(OutputType);

impl<'de> serde::Deserialize<'de> for InternalOutputType {
    fn deserialize<D>(deserializer: D) -> Result<InternalOutputType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl FromStr for InternalOutputType {
    type Err = roles_logic_sv2::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "P2PK" => Ok(Self(OutputType::Bare)),
            "P2PKH" => Ok(Self(OutputType::Bare)),
            "P2WPKH" => Ok(Self(OutputType::Wpkh)),
            "P2SH" => Ok(Self(OutputType::Sh)),
            "P2WSH" => Ok(Self(OutputType::Wsh)),
            "P2TR" => Ok(Self(OutputType::Tr)),
            _ => Err(roles_logic_sv2::Error::UnknownOutputScriptType),
        }
    }
}

/// Represents a coinbase output.
#[derive(Debug, Clone)]
pub struct CoinbaseOutput {
    script_type: InternalOutputType,
    script_value: ScriptBuf,
}

impl CoinbaseOutput {
    /// Creates a new [`CoinbaseOutput`].
    pub fn new(script_type: String, script_value: String) -> Result<Self, Error> {
        let script_value: ScriptBuf = roles_logic_sv2::utils::CoinbaseOutput {
            output_script_type: script_type.clone(),
            output_script_value: script_value,
        }
        .try_into()
        .map_err(|e| {
            error!("Error converting script value: {:?}", e);
            Error::InvalidCoinbaseOutput
        })?;
        let script_type = InternalOutputType::from_str(&script_type).map_err(|e| {
            error!("Error converting script type: {:?}", e);
            Error::InvalidCoinbaseOutput
        })?;
        Ok(Self {
            script_type,
            script_value,
        })
    }

    /// Returns the [`TxOut`] of the [`CoinbaseOutput`].
    ///
    /// Note that the value of the [`TxOut`] is always 0.
    pub fn get_txout(&self) -> Result<TxOut, roles_logic_sv2::Error> {
        Ok(TxOut {
            value: Amount::from_sat(0),
            script_pubkey: self.script_value.clone(),
        })
    }

    /// Returns the script type of the coinbase output.
    pub fn output_type(&self) -> OutputType {
        self.script_type.0
    }

    /// Returns the script value of the coinbase output.
    pub fn output_script(&self) -> &ScriptBuf {
        &self.script_value
    }
}
