use roles_logic_sv2::utils::CoinbaseOutput as CoinbaseOutput_;
use std::convert::TryFrom;

#[derive(Debug, serde::Deserialize, Clone)]
pub struct CoinbaseOutput {
    output_script_type: String,
    output_script_value: String,
}

impl CoinbaseOutput {
    pub fn new(output_script_type: String, output_script_value: String) -> Self {
        Self {
            output_script_type,
            output_script_value,
        }
    }
}

impl TryFrom<&CoinbaseOutput> for CoinbaseOutput_ {
    type Error = roles_logic_sv2::Error;

    fn try_from(pool_output: &CoinbaseOutput) -> Result<Self, Self::Error> {
        match pool_output.output_script_type.as_str() {
            "P2PK" | "P2PKH" | "P2WPKH" | "P2SH" | "P2WSH" | "P2TR" => Ok(CoinbaseOutput_ {
                output_script_type: pool_output.clone().output_script_type,
                output_script_value: pool_output.clone().output_script_value,
            }),
            _ => Err(roles_logic_sv2::Error::UnknownOutputScriptType),
        }
    }
}
