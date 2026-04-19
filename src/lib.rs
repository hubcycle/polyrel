#![cfg_attr(not(feature = "std"), no_std)]
//! Typed building blocks for Polymarket relayer payloads.
//!
//! The crate is split into two layers:
//!
//! - generic calldata builders in modules like [`erc20`], [`erc1155`], and [`ctf`]
//! - Safe-specific payload construction in [`safe`]
//!
//! The optional [`client`] module adds a raw relayer HTTP client behind the
//! `client` feature.
//!
//! # Examples
//!
//! Build a gasless Safe execution draft for an ERC-20 approval:
//!
//! ```no_run
//! use alloy_primitives::{Address, U256, address, b256};
//! use polyrel::{
//!     NonEmptyCalls, erc20,
//!     safe::{
//!         ChainId, SafeExecutionContext, SafeGasParams, SafeNonce, build_execution_draft,
//!     },
//! };
//!
//! let call = erc20::approve(
//!     address!("2791Bca1f2de4661ED88A30C99A7a9449Aa84174"),
//!     address!("4d97dcd97ec945f40cf65f87097ace5ea0476045"),
//!     U256::from(1_000_000_u64),
//! );
//! let context = SafeExecutionContext::builder()
//!     .owner(address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5"))
//!     .chain_id(ChainId::new(137.try_into().unwrap()))
//!     .safe_factory(address!("aacfeea03eb1561c4e67d661e40682bd20e3541b"))
//!     .safe_init_code_hash(
//!         b256!("2bce2127ff07fb632d16c8347c4ebf501f4841168bed00d9e6ef715ddb6fcecf"),
//!     )
//!     .safe_multisend(address!("a238cbeb142c10ef7ad8442c6d1f9e89e07e7761"))
//!     .nonce(SafeNonce::new(U256::ZERO))
//!     .gas_params(
//!         SafeGasParams::builder()
//!             .safe_txn_gas(U256::ZERO)
//!             .base_gas(U256::ZERO)
//!             .gas_price(U256::ZERO)
//!             .gas_token(Address::ZERO)
//!             .refund_receiver(Address::ZERO)
//!             .build(),
//!     )
//!     .build();
//! let _draft = build_execution_draft(&context, NonEmptyCalls::from_one(call)).unwrap();
//! ```
//!
//! Create a relayer client when the `client` feature is enabled:
//!
//! ```no_run
//! # #[cfg(feature = "client")]
//! # async fn demo() -> Result<(), polyrel::PolyrelError> {
//! use alloy_primitives::address;
//! use polyrel::client::{RelayerApiKeyAuth, RelayerBaseUrl, RelayerClient};
//! use secrecy::SecretString;
//!
//! let base_url = RelayerBaseUrl::parse("https://relayer-v2.polymarket.com")?;
//! let _client = RelayerClient::new(base_url).authenticate_relayer(RelayerApiKeyAuth::new(
//!     SecretString::from("replace-me"),
//!     address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5"),
//! ));
//! # Ok(())
//! # }
//! ```

extern crate alloc;

/// Raw Polymarket relayer HTTP client and authenticated transport helpers.
pub mod client;
/// Conditional Tokens Framework calldata builders.
pub mod ctf;
/// ERC-1155 calldata builders.
pub mod erc1155;
/// ERC-20 calldata builders.
pub mod erc20;
/// Neg-risk adapter calldata builders.
pub mod neg_risk;
/// Polymarket-specific approval recipes built on top of the generic token helpers.
pub mod polymarket;
/// Safe-specific payload construction for deployment and execution requests.
pub mod safe;

mod call;
mod error;

/// Generic EVM call envelope used throughout the crate.
pub use call::Call;
/// Non-empty collection of [`Call`] values.
pub use call::NonEmptyCalls;
/// Error type used by the crate.
pub use error::PolyrelError;
