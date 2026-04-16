//! Core types shared across the crate.

use core::str::FromStr;

use std::borrow::Cow;

use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::PolyrelError;

/// Configuration for contract addresses and relayer defaults.
pub struct Config {
	base_url: Url,
	chain_id: u64,
	ctf_exchange: Address,
	neg_risk_ctf_exchange: Address,
	neg_risk_adapter: Address,
	conditional_tokens: Address,
	usdc_e: Address,
	proxy_wallet_factory: Address,
	relay_hub: Address,
	safe_factory: Address,
	safe_multisend: Address,
	safe_init_code_hash: B256,
	proxy_init_code_hash: B256,
}

#[bon::bon]
impl Config {
	/// Build a new configuration; all fields default to Polygon mainnet values.
	#[builder]
	pub fn new(
		base_url: Option<Cow<'static, str>>,
		chain_id: Option<u64>,
		ctf_exchange: Option<Address>,
		neg_risk_ctf_exchange: Option<Address>,
		neg_risk_adapter: Option<Address>,
		conditional_tokens: Option<Address>,
		usdc_e: Option<Address>,
		proxy_wallet_factory: Option<Address>,
		relay_hub: Option<Address>,
		safe_factory: Option<Address>,
		safe_multisend: Option<Address>,
		safe_init_code_hash: Option<B256>,
		proxy_init_code_hash: Option<B256>,
	) -> Result<Self, crate::PolyrelError> {
		let url_str = base_url.as_deref().unwrap_or(crate::RELAYER_BASE_URL);
		let mut parsed = Url::parse(url_str)
			.map_err(|e| crate::PolyrelError::http(format!("invalid base URL: {e}")))?;
		match parsed.scheme() {
			"http" | "https" => {},
			_ => {
				return Err(crate::PolyrelError::http(
					"base URL must use http or https scheme",
				));
			},
		}
		if parsed.query().is_some() {
			return Err(crate::PolyrelError::http(
				"base URL must not contain a query string",
			));
		}
		if parsed.fragment().is_some() {
			return Err(crate::PolyrelError::http(
				"base URL must not contain a fragment",
			));
		}
		let trimmed = parsed.path().trim_end_matches('/').to_owned();
		parsed.set_path(&trimmed);

		Ok(Self {
			base_url: parsed,
			chain_id: chain_id.unwrap_or(crate::CHAIN_ID),
			ctf_exchange: ctf_exchange.unwrap_or(crate::CTF_EXCHANGE),
			neg_risk_ctf_exchange: neg_risk_ctf_exchange.unwrap_or(crate::NEG_RISK_CTF_EXCHANGE),
			neg_risk_adapter: neg_risk_adapter.unwrap_or(crate::NEG_RISK_ADAPTER),
			conditional_tokens: conditional_tokens.unwrap_or(crate::CONDITIONAL_TOKENS),
			usdc_e: usdc_e.unwrap_or(crate::USDC_E),
			proxy_wallet_factory: proxy_wallet_factory.unwrap_or(crate::PROXY_WALLET_FACTORY),
			relay_hub: relay_hub.unwrap_or(crate::RELAY_HUB),
			safe_factory: safe_factory.unwrap_or(crate::SAFE_FACTORY),
			safe_multisend: safe_multisend.unwrap_or(crate::SAFE_MULTISEND),
			safe_init_code_hash: safe_init_code_hash
				.unwrap_or_else(|| crate::SAFE_INIT_CODE_HASH.into()),
			proxy_init_code_hash: proxy_init_code_hash
				.unwrap_or_else(|| crate::PROXY_INIT_CODE_HASH.into()),
		})
	}

	/// Relayer API base URL.
	pub fn base_url(&self) -> &Url {
		&self.base_url
	}

	/// Chain ID.
	pub fn chain_id(&self) -> u64 {
		self.chain_id
	}

	/// CTF Exchange address.
	pub fn ctf_exchange(&self) -> Address {
		self.ctf_exchange
	}

	/// Neg-Risk CTF Exchange address.
	pub fn neg_risk_ctf_exchange(&self) -> Address {
		self.neg_risk_ctf_exchange
	}

	/// Neg-Risk Adapter address.
	pub fn neg_risk_adapter(&self) -> Address {
		self.neg_risk_adapter
	}

	/// Conditional Tokens address.
	pub fn conditional_tokens(&self) -> Address {
		self.conditional_tokens
	}

	/// USDC.e address.
	pub fn usdc_e(&self) -> Address {
		self.usdc_e
	}

	/// Proxy Wallet Factory address.
	pub fn proxy_wallet_factory(&self) -> Address {
		self.proxy_wallet_factory
	}

	/// GSN Relay Hub address.
	pub fn relay_hub(&self) -> Address {
		self.relay_hub
	}

	/// Gnosis Safe Factory address.
	pub fn safe_factory(&self) -> Address {
		self.safe_factory
	}

	/// Safe MultiSend address.
	pub fn safe_multisend(&self) -> Address {
		self.safe_multisend
	}

	/// Safe init code hash for CREATE2 derivation.
	pub fn safe_init_code_hash(&self) -> B256 {
		self.safe_init_code_hash
	}

	/// Proxy init code hash for CREATE2 derivation.
	pub fn proxy_init_code_hash(&self) -> B256 {
		self.proxy_init_code_hash
	}
}

/// Known transaction lifecycle states emitted by the relayer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownTransactionState {
	/// Queued for processing.
	#[serde(rename = "STATE_NEW")]
	New,

	/// Submitted to blockchain.
	#[serde(rename = "STATE_EXECUTED")]
	Executed,

	/// Included in a block.
	#[serde(rename = "STATE_MINED")]
	Mined,

	/// Finalized (~30 blocks).
	#[serde(rename = "STATE_CONFIRMED")]
	Confirmed,

	/// Invalid transaction.
	#[serde(rename = "STATE_INVALID")]
	Invalid,

	/// Execution failed.
	#[serde(rename = "STATE_FAILED")]
	Failed,
}

/// Transaction lifecycle state as returned by the relayer.
///
/// The untagged representation preserves unknown wire values in
/// [`TransactionState::Unknown`] so the client does not fail deserialization
/// if the relayer introduces new states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransactionState {
	/// One of the states the client recognises.
	Known(KnownTransactionState),

	/// An unrecognised state string received from the relayer.
	Unknown(String),
}

impl TransactionState {
	/// Return `true` if this state equals the given known state.
	pub fn is(&self, known: KnownTransactionState) -> bool {
		matches!(self, Self::Known(k) if *k == known)
	}
}

/// Transaction nonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nonce(U256);

impl Nonce {
	/// Wrap a raw [`U256`] nonce.
	pub fn new(value: U256) -> Self {
		Self(value)
	}

	/// Return the underlying [`U256`].
	pub fn raw(self) -> U256 {
		self.0
	}
}

impl Serialize for Nonce {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_str(&self.0)
	}
}

impl<'de> Deserialize<'de> for Nonce {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let raw = String::deserialize(deserializer)?;
		U256::from_str(&raw).map(Self).map_err(serde::de::Error::custom)
	}
}

/// Unique identifier for a relayer transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionId(String);

impl TransactionId {
	/// Validate and wrap a transaction identifier.
	pub fn new(id: impl Into<String>) -> Result<Self, PolyrelError> {
		let id = id.into();
		if id.is_empty() {
			return Err(PolyrelError::deserialize("transaction id cannot be empty"));
		}
		Ok(Self(id))
	}

	/// Borrow as `&str`.
	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl Serialize for TransactionId {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.0)
	}
}

impl<'de> Deserialize<'de> for TransactionId {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let raw = String::deserialize(deserializer)?;
		Self::new(raw).map_err(serde::de::Error::custom)
	}
}

/// Wallet type for relayer transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalletType {
	/// Gnosis Safe wallet.
	#[serde(rename = "SAFE")]
	Safe,

	/// Polymarket proxy wallet.
	#[serde(rename = "PROXY")]
	Proxy,

	/// Safe creation (deployment).
	#[serde(rename = "SAFE-CREATE")]
	SafeCreate,
}

/// Safe transaction operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OperationType {
	/// Standard call.
	Call = 0,

	/// Delegate call (used by MultiSend).
	DelegateCall = 1,
}

impl OperationType {
	/// Raw u8 value.
	pub fn as_u8(self) -> u8 {
		self as u8
	}
}

/// Parameters for the relayer transaction signature.
///
/// Different transaction types use different subsets of fields.
/// Safe transactions use the gas fields. Safe-create uses
/// payment fields. Proxy transactions use relay fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureParams {
	/// Gas price (Safe transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub gas_price: Option<String>,

	/// Operation type: 0 = Call, 1 = DelegateCall (Safe transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub operation: Option<String>,

	/// Gas allocated for the Safe transaction.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub safe_txn_gas: Option<String>,

	/// Base gas overhead (Safe transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub base_gas: Option<String>,

	/// Token used for gas payment (Safe transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub gas_token: Option<String>,

	/// Address receiving gas refund (Safe transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub refund_receiver: Option<String>,

	/// Payment token (Safe-create transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub payment_token: Option<String>,

	/// Payment amount (Safe-create transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub payment: Option<String>,

	/// Payment receiver (Safe-create transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub payment_receiver: Option<String>,

	/// Relayer fee (Proxy transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relayer_fee: Option<String>,

	/// Gas limit (Proxy transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub gas_limit: Option<String>,

	/// Relay hub address (Proxy transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relay_hub: Option<String>,

	/// Relay address (Proxy transactions).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relay: Option<String>,
}

impl SignatureParams {
	/// Params for a Safe transaction.
	pub fn safe(operation: u8) -> Self {
		Self {
			gas_price: Some("0".to_owned()),
			operation: Some(operation.to_string()),
			safe_txn_gas: Some("0".to_owned()),
			base_gas: Some("0".to_owned()),
			gas_token: Some(Address::ZERO.to_string()),
			refund_receiver: Some(Address::ZERO.to_string()),
			..Default::default()
		}
	}

	/// Params for a Safe-create (deployment) transaction.
	pub fn safe_create() -> Self {
		Self {
			payment_token: Some(Address::ZERO.to_string()),
			payment: Some("0".to_owned()),
			payment_receiver: Some(Address::ZERO.to_string()),
			..Default::default()
		}
	}

	/// Params for a Proxy transaction.
	pub fn proxy(
		gas_price: Cow<'static, str>,
		gas_limit: Cow<'static, str>,
		relay_hub: Address,
		relay: Address,
	) -> Self {
		Self {
			gas_price: Some(gas_price.into_owned()),
			gas_limit: Some(gas_limit.into_owned()),
			relayer_fee: Some("0".to_owned()),
			relay_hub: Some(relay_hub.to_string()),
			relay: Some(relay.to_string()),
			..Default::default()
		}
	}
}

/// Request body for `POST /submit`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRequest {
	/// Transaction type.
	#[serde(rename = "type")]
	pub wallet_type: WalletType,

	/// Signer address.
	pub from: String,

	/// Target contract address.
	pub to: String,

	/// User's proxy wallet address.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub proxy_wallet: Option<String>,

	/// Hex-encoded transaction data.
	pub data: String,

	/// Transaction nonce.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub nonce: Option<String>,

	/// Hex-encoded signature.
	pub signature: String,

	/// Signature parameters.
	pub signature_params: SignatureParams,

	/// Optional metadata.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<String>,
}

#[bon::bon]
impl SubmitRequest {
	/// Build a new submit request.
	#[builder]
	pub fn new(
		wallet_type: WalletType,
		from: Cow<'static, str>,
		to: Cow<'static, str>,
		proxy_wallet: Option<Cow<'static, str>>,
		data: Cow<'static, str>,
		nonce: Option<Cow<'static, str>>,
		signature: Cow<'static, str>,
		signature_params: SignatureParams,
		metadata: Option<Cow<'static, str>>,
	) -> Self {
		Self {
			wallet_type,
			from: from.into_owned(),
			to: to.into_owned(),
			proxy_wallet: proxy_wallet.map(Cow::into_owned),
			data: data.into_owned(),
			nonce: nonce.map(Cow::into_owned),
			signature: signature.into_owned(),
			signature_params,
			metadata: metadata.map(Cow::into_owned),
		}
	}
}

/// Response from `POST /submit`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubmitResponse {
	/// Unique transaction identifier.
	#[serde(rename = "transactionID")]
	pub transaction_id: TransactionId,

	/// Current state.
	pub state: TransactionState,

	/// On-chain transaction hash.
	#[serde(default)]
	pub hash: Option<String>,

	/// On-chain transaction hash (alias).
	#[serde(rename = "transactionHash", default)]
	pub transaction_hash: Option<String>,
}

/// Full transaction record from `GET /transaction`.
#[derive(Debug, Clone, Deserialize)]
pub struct RelayerTransaction {
	/// Unique transaction identifier.
	#[serde(rename = "transactionID")]
	pub transaction_id: TransactionId,

	/// On-chain transaction hash.
	#[serde(rename = "transactionHash", default)]
	pub transaction_hash: Option<String>,

	/// Sender address.
	#[serde(default)]
	pub from: Option<String>,

	/// Target contract address.
	#[serde(default)]
	pub to: Option<String>,

	/// Proxy/Safe address.
	#[serde(rename = "proxyAddress", default)]
	pub proxy_address: Option<String>,

	/// Transaction data.
	#[serde(default)]
	pub data: Option<String>,

	/// Transaction nonce.
	#[serde(default)]
	pub nonce: Option<Nonce>,

	/// ETH value.
	#[serde(default)]
	pub value: Option<String>,

	/// Current state.
	pub state: TransactionState,

	/// Transaction type.
	#[serde(rename = "type", default)]
	pub transaction_type: Option<String>,

	/// Metadata.
	#[serde(default)]
	pub metadata: Option<String>,

	/// Signature.
	#[serde(default)]
	pub signature: Option<String>,

	/// Owner / API key owner.
	#[serde(default)]
	pub owner: Option<String>,

	/// Creation timestamp.
	#[serde(rename = "createdAt", default)]
	pub created_at: Option<String>,

	/// Last update timestamp.
	#[serde(rename = "updatedAt", default)]
	pub updated_at: Option<String>,
}

/// Response from `GET /relay-payload`.
#[derive(Debug, Clone, Deserialize)]
pub struct RelayerInfo {
	/// Relayer's address.
	pub address: String,

	/// Current nonce.
	pub nonce: Nonce,
}

/// Response from `GET /deployed`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeployedResponse {
	/// Whether the Safe is deployed.
	pub deployed: bool,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn config_default_succeeds() {
		// Act
		let config = Config::builder().build();

		// Assert
		assert!(config.is_ok());
	}

	#[test]
	fn config_rejects_ftp_scheme() {
		// Act
		let result = Config::builder().base_url("ftp://example.com".into()).build();

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn config_rejects_mailto_scheme() {
		// Act
		let result = Config::builder().base_url("mailto:test@example.com".into()).build();

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn config_rejects_query_string() {
		// Act
		let result = Config::builder().base_url("https://example.com?key=val".into()).build();

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn config_rejects_fragment() {
		// Act
		let result = Config::builder().base_url("https://example.com#frag".into()).build();

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn config_normalizes_single_trailing_slash() {
		// Arrange & Act
		let config = Config::builder().base_url("https://example.com/api/".into()).build().unwrap();

		// Assert
		assert!(!config.base_url().as_str().ends_with('/'));
	}

	#[test]
	fn config_normalizes_multiple_trailing_slashes() {
		// Arrange & Act
		let config =
			Config::builder().base_url("https://example.com/api///".into()).build().unwrap();

		// Assert
		assert_eq!(config.base_url().path(), "/api");
	}

	#[test]
	fn config_accepts_valid_https_url() {
		// Arrange & Act
		let config =
			Config::builder().base_url("https://relayer.example.com".into()).build().unwrap();

		// Assert
		assert_eq!(config.base_url().scheme(), "https");
	}

	#[test]
	fn signature_params_safe_sets_gas_defaults() {
		// Act
		let params = SignatureParams::safe(0);

		// Assert
		assert_eq!(params.gas_price.as_deref(), Some("0"));
		assert_eq!(params.operation.as_deref(), Some("0"));
		assert_eq!(params.safe_txn_gas.as_deref(), Some("0"));
		assert_eq!(params.base_gas.as_deref(), Some("0"));
		assert!(params.payment_token.is_none());
	}

	#[test]
	fn signature_params_safe_create_sets_payment_defaults() {
		// Act
		let params = SignatureParams::safe_create();

		// Assert
		assert!(params.payment_token.is_some());
		assert_eq!(params.payment.as_deref(), Some("0"));
		assert!(params.payment_receiver.is_some());
		assert!(params.gas_price.is_none());
	}

	#[test]
	fn submit_response_deserializes_transaction_id_field() {
		// Arrange
		let json = r#"{"transactionID":"abc-123","state":"STATE_NEW"}"#;

		// Act
		let resp: SubmitResponse = serde_json::from_str(json).unwrap();

		// Assert
		assert_eq!(resp.transaction_id.as_str(), "abc-123");
		assert!(resp.state.is(KnownTransactionState::New));
	}

	#[test]
	fn relayer_transaction_deserializes_full_payload() {
		// Arrange
		let json = r#"{
			"transactionID": "tx-1",
			"transactionHash": "0xabc",
			"from": "0x1234",
			"to": "0x5678",
			"proxyAddress": "0xproxy",
			"state": "STATE_MINED",
			"signature": "0xsig",
			"owner": "owner-uuid"
		}"#;

		// Act
		let txn: RelayerTransaction = serde_json::from_str(json).unwrap();

		// Assert
		assert_eq!(txn.transaction_id.as_str(), "tx-1");
		assert_eq!(txn.signature.as_deref(), Some("0xsig"));
		assert_eq!(txn.owner.as_deref(), Some("owner-uuid"));
	}

	#[test]
	fn wallet_type_serializes_safe_create_with_hyphen() {
		// Act
		let json = serde_json::to_string(&WalletType::SafeCreate).unwrap();

		// Assert
		assert_eq!(json, "\"SAFE-CREATE\"");
	}

	#[test]
	fn transaction_state_known_round_trips() {
		// Arrange
		let json = "\"STATE_CONFIRMED\"";

		// Act
		let state: TransactionState = serde_json::from_str(json).unwrap();

		// Assert
		assert_eq!(
			state,
			TransactionState::Known(KnownTransactionState::Confirmed)
		);
		assert_eq!(serde_json::to_string(&state).unwrap(), json);
	}

	#[test]
	fn transaction_state_unknown_preserved() {
		// Arrange
		let json = "\"STATE_PENDING_RETRY\"";

		// Act
		let state: TransactionState = serde_json::from_str(json).unwrap();

		// Assert
		assert_eq!(
			state,
			TransactionState::Unknown("STATE_PENDING_RETRY".to_owned())
		);
	}
}
