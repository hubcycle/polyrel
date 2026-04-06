//! Unofficial Polymarket relayer client for gasless transactions.
//!
//! This crate provides a typed Rust client for the
//! [Polymarket relayer API](https://docs.polymarket.com/trading/gasless),
//! enabling gasless on-chain operations through Safe and Proxy wallets.
//!
//! # Client construction
//!
//! The client uses a typestate pattern: start unauthenticated, then
//! attach credentials to unlock submission methods.
//!
//! ```
//! use polyrel::{RelayerClient, Auth, BuilderCredentials};
//! use secrecy::SecretString;
//!
//! let client = RelayerClient::builder()
//!     .build()
//!     .expect("default config is valid");
//!
//! let auth = Auth::Builder(BuilderCredentials {
//!     api_key: SecretString::from("key"),
//!     secret: SecretString::from("c2VjcmV0"),
//!     passphrase: SecretString::from("pass"),
//! });
//! let client = client.authenticate(auth);
//! ```
//!
//! Override defaults for testnets or custom deployments. When
//! pointing at a non-Polygon deployment, override all contract
//! addresses and init-code hashes that differ — `base_url` and
//! `chain_id` alone are not sufficient:
//!
//! ```
//! use alloy_primitives::{address, B256};
//! use polyrel::RelayerClient;
//!
//! let client = RelayerClient::builder()
//!     .base_url("https://relayer-testnet.example.com".into())
//!     .chain_id(80002_u64)
//!     .safe_factory(address!("aacFeEa03eb1561C4e67d661e40682Bd20E3541b"))
//!     .safe_multisend(address!("A238CBeb142c10Ef7Ad8442C6D1f9E89e07e7761"))
//!     .safe_init_code_hash(B256::from(polyrel::SAFE_INIT_CODE_HASH))
//!     .build()
//!     .expect("valid config");
//! ```
//!
//! # Wallet address derivation
//!
//! Derive deterministic Safe or Proxy wallet addresses from an owner
//! and factory using CREATE2:
//!
//! ```
//! use alloy_primitives::{address, Address, B256};
//! use polyrel::{derive_safe_address, derive_proxy_address};
//!
//! let owner = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
//! let safe_factory = address!("aacFeEa03eb1561C4e67d661e40682Bd20E3541b");
//! let proxy_factory = address!("aB45c5A4B0c941a2F231C04C3f49182e1A254052");
//! let safe_hash = B256::from(polyrel::SAFE_INIT_CODE_HASH);
//! let proxy_hash = B256::from(polyrel::PROXY_INIT_CODE_HASH);
//!
//! let safe_addr = derive_safe_address(owner, safe_factory, safe_hash);
//! let proxy_addr = derive_proxy_address(owner, proxy_factory, proxy_hash);
//!
//! assert_ne!(safe_addr, Address::ZERO);
//! assert_ne!(proxy_addr, Address::ZERO);
//! ```
//!
//! # Configuration
//!
//! [`Config`] holds all contract addresses, init-code hashes, and the
//! relayer base URL. It defaults to Polygon mainnet values and can be
//! inspected after construction:
//!
//! ```
//! use polyrel::Config;
//!
//! let config = Config::builder().build().unwrap();
//! assert_eq!(config.chain_id(), 137);
//! assert_eq!(config.base_url().scheme(), "https");
//! ```
//!
//! # Batching with MultiSend
//!
//! Multiple Safe transactions can be aggregated into a single
//! MultiSend delegate call:
//!
//! ```
//! use alloy_primitives::{Address, U256};
//! use polyrel::{SafeTransaction, NonEmptyTransactions, OperationType, aggregate_transactions};
//!
//! let tx1 = SafeTransaction {
//!     to: Address::ZERO,
//!     value: U256::ZERO,
//!     data: vec![0x01],
//!     operation: OperationType::Call,
//! };
//! let tx2 = SafeTransaction {
//!     to: Address::ZERO,
//!     value: U256::ZERO,
//!     data: vec![0x02],
//!     operation: OperationType::Call,
//! };
//! let batch = NonEmptyTransactions::new(vec![tx1, tx2]).unwrap();
//! let multisend_addr = Address::ZERO; // use config.safe_multisend() in practice
//! let combined = aggregate_transactions(batch, multisend_addr);
//!
//! assert_eq!(combined.operation, OperationType::DelegateCall);
//! ```
//!
//! # Safe signature packing
//!
//! Raw ECDSA signatures must be packed into Safe's expected format
//! before submission. The v-value is adjusted: `0/1 → +31`, `27/28 → +4`.
//!
//! ```
//! use polyrel::pack_safe_signature;
//!
//! // 64 bytes of r+s, then v=27
//! let mut sig = vec![0xaa; 64];
//! sig.push(27);
//! let hex = alloy_primitives::hex::encode(&sig);
//!
//! let packed = pack_safe_signature(&hex).unwrap();
//! let bytes = alloy_primitives::hex::decode(packed.strip_prefix("0x").unwrap()).unwrap();
//! assert_eq!(bytes[64], 31); // 27 + 4
//! ```
//!
//! # Submitting transactions
//!
//! Authenticated clients can sign and submit Safe transactions. The
//! Safe address is derived automatically from the signer and factory:
//!
//! ```no_run
//! use polyrel::{RelayerClient, Auth, BuilderCredentials};
//! use secrecy::SecretString;
//! use alloy_primitives::U256;
//! use alloy_signer::Signer;
//!
//! async fn run(signer: &(impl Signer + Sync)) -> Result<(), polyrel::PolyrelError> {
//!     let client = RelayerClient::builder().build()?
//!         .authenticate(Auth::Builder(BuilderCredentials {
//!             api_key: SecretString::from("key"),
//!             secret: SecretString::from("c2VjcmV0"),
//!             passphrase: SecretString::from("pass"),
//!         }));
//!
//!     // fetch the current nonce for this signer's Safe wallet
//!     let nonce_str = client.safe_nonce(signer.address()).await?;
//!     let nonce = nonce_str.parse::<U256>().expect("valid nonce");
//!
//!     // approve USDC for the CTF Exchange
//!     let resp = client
//!         .approve_usdc_for_exchange(signer, U256::MAX, nonce)
//!         .await?;
//!
//!     // poll until confirmed
//!     let txn = client
//!         .poll_until_state(
//!             &resp.transaction_id,
//!             &["STATE_MINED", "STATE_CONFIRMED"],
//!             Some("STATE_FAILED"),
//!             None,
//!             None,
//!         )
//!         .await?;
//!     Ok(())
//! }

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

/// Gnosis Safe Factory (Polygon mainnet default).
pub const SAFE_FACTORY: Address = address!("aacFeEa03eb1561C4e67d661e40682Bd20E3541b");

pub(crate) const SAFE_MULTISEND: Address = address!("A238CBeb142c10Ef7Ad8442C6D1f9E89e07e7761");

/// Safe init code hash for CREATE2 derivation (Polygon mainnet default).
pub const SAFE_INIT_CODE_HASH: [u8; 32] =
	alloy_primitives::hex!("2bce2127ff07fb632d16c8347c4ebf501f4841168bed00d9e6ef715ddb6fcecf");

/// Proxy init code hash for CREATE2 derivation (Polygon mainnet default).
pub const PROXY_INIT_CODE_HASH: [u8; 32] =
	alloy_primitives::hex!("d21df8dc65880a8606f09fe0ce3df9b8869287ab0b058be05aa9e8af6330a00b");
