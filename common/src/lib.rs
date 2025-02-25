//! # Stratum Common Crate
//!
//! `stratum_common` is a utility crate designed to centralize
//! and manage the shared dependencies and utils across stratum crates.
#[cfg(feature = "bitcoin")]
pub use bitcoin;
pub use secp256k1;
pub mod coinbase_output;

#[cfg(feature = "serde")]
pub mod toml;
#[cfg(feature = "serde")]
pub mod url;
