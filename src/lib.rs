//! Unofficial Polymarket relayer client.

mod auth;
mod client;
mod error;
mod sign;
mod types;

pub use auth::{Auth, BuilderCredentials, RelayerApiKey};
pub use client::{Authenticated, RelayerClient, Unauthenticated};
pub use error::PolyrelError;
pub use sign::{
	NonEmptyProxyCalls, NonEmptyTransactions, ProxyTransactionArgs, SafeTransaction,
	aggregate_transactions, derive_proxy_address, derive_safe_address, encode_proxy_calls,
	neg_risk_redeem_positions, pack_safe_signature, sign_proxy_transaction,
};
pub use types::{
	Config, DeployedResponse, OperationType, RelayerInfo, RelayerTransaction, SignatureParams,
	SubmitRequest, SubmitResponse, TransactionState, WalletType,
};

use alloy_primitives::{Address, address};

/// Safe factory EIP-712 domain name for `CreateProxy` typed data.
pub const SAFE_FACTORY_NAME: &str = "Polymarket Contract Proxy Factory";

pub(crate) const CHAIN_ID: u64 = 137;
pub(crate) const RELAYER_BASE_URL: &str = "https://relayer-v2.polymarket.com";

pub(crate) const CTF_EXCHANGE: Address = address!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E");

pub(crate) const NEG_RISK_CTF_EXCHANGE: Address =
	address!("C5d563A36AE78145C45a50134d48A1215220f80a");

pub(crate) const CONDITIONAL_TOKENS: Address = address!("4D97DCd97eC945f40cF65F87097ACe5EA0476045");

pub(crate) const USDC_E: Address = address!("2791Bca1f2de4661ED88A30C99A7a9449Aa84174");

pub(crate) const PROXY_WALLET_FACTORY: Address =
	address!("aB45c5A4B0c941a2F231C04C3f49182e1A254052");

pub(crate) const RELAY_HUB: Address = address!("D216153c06E857cD7f72665E0aF1d7D82172F494");

pub(crate) const SAFE_FACTORY: Address = address!("aacFeEa03eb1561C4e67d661e40682Bd20E3541b");

pub(crate) const SAFE_MULTISEND: Address = address!("A238CBeb142c10Ef7Ad8442C6D1f9E89e07e7761");

pub(crate) const SAFE_INIT_CODE_HASH: [u8; 32] =
	alloy_primitives::hex!("2bce2127ff07fb632d16c8347c4ebf501f4841168bed00d9e6ef715ddb6fcecf");

pub(crate) const PROXY_INIT_CODE_HASH: [u8; 32] =
	alloy_primitives::hex!("d21df8dc65880a8606f09fe0ce3df9b8869287ab0b058be05aa9e8af6330a00b");
