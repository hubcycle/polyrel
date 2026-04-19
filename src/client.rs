#![cfg(feature = "client")]
//! Raw Polymarket relayer HTTP client.

mod dto;

use alloc::{borrow::Cow, string::String, vec::Vec};

use core::str::FromStr;

use std::time::{SystemTime, UNIX_EPOCH};

use alloy_primitives::{Address, B256, Bytes, U256};
use base64::{
	Engine as _,
	engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
};
use hmac::{Hmac, Mac};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sha2::Sha256;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use url::Url;
use uuid::Uuid;

use crate::{PolyrelError, safe::SubmitRequest};

mod sealed {
	use super::{HeaderMap, PolyrelError};

	pub trait Authenticated {
		fn headers(&self, method: &str, path: &str, body: &str) -> Result<HeaderMap, PolyrelError>;
	}
}

/// Typestate marker for an unauthenticated relayer client.
pub struct Unauthenticated;

/// Typestate marker for a relayer client authenticated with a relayer API key.
pub struct RelayerAuthenticated {
	auth: RelayerApiKeyAuth,
}

/// Typestate marker for a relayer client authenticated with builder HMAC headers.
pub struct BuilderAuthenticated {
	auth: BuilderAuth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Query kind used for relayer endpoints that distinguish Safe and proxy wallets.
pub enum WalletQueryKind {
	/// Query the Safe wallet view.
	Safe,
	/// Query the proxy wallet view.
	Proxy,
}

#[derive(Clone)]
/// Relayer API key authentication material.
pub struct RelayerApiKeyAuth {
	key: SecretString,
	address: Address,
}

#[derive(Clone)]
/// Builder HMAC authentication material.
pub struct BuilderAuth {
	key: SecretString,
	secret: SecretString,
	passphrase: SecretString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Validated base URL for the relayer service.
pub struct RelayerBaseUrl {
	url: Url,
}

/// Typed relayer client parameterized by authentication state.
pub struct RelayerClient<State = Unauthenticated> {
	base_url: RelayerBaseUrl,
	http: reqwest::Client,
	state: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Documented relayer transaction lifecycle states.
pub enum RelayerTransactionState {
	/// Transaction was received by the relayer.
	New,
	/// Transaction was submitted onchain.
	Executed,
	/// Transaction was included in a block.
	Mined,
	/// Transaction was finalized successfully.
	Confirmed,
	/// Transaction failed permanently.
	Failed,
	/// Transaction was rejected as invalid.
	Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Transaction kinds accepted by the relayer.
pub enum RelayerTransactionKind {
	/// Standard Safe execution request.
	Safe,
	/// Safe deployment request.
	SafeCreate,
	/// Proxy transaction request.
	Proxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// UUID-backed relayer transaction identifier.
pub struct TransactionId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// UUID-backed relayer API key identifier.
pub struct RelayerApiKeyId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
/// Non-empty transaction metadata string returned by the relayer.
pub struct TransactionMetadata(Cow<'static, str>);

#[derive(Debug, Clone, PartialEq, Eq)]
/// Current Safe or proxy nonce returned by the relayer.
pub struct CurrentNonce {
	nonce: U256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Relayer payload details used by proxy flows.
pub struct RelayPayload {
	address: Address,
	nonce: U256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Result of submitting a transaction to the relayer.
pub struct SubmittedTransaction {
	transaction_id: TransactionId,
	state: RelayerTransactionState,
	hash: Option<B256>,
	transaction_hash: Option<B256>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Relayer transaction record returned by listing and lookup endpoints.
pub struct RelayerTransaction {
	transaction_id: TransactionId,
	transaction_hash: Option<B256>,
	from: Option<Address>,
	to: Option<Address>,
	proxy_address: Option<Address>,
	data: Option<Bytes>,
	nonce: Option<U256>,
	value: Option<U256>,
	state: RelayerTransactionState,
	transaction_kind: Option<RelayerTransactionKind>,
	metadata: Option<TransactionMetadata>,
	signature: Option<Bytes>,
	owner: Option<Address>,
	created_at: Option<OffsetDateTime>,
	updated_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Relayer API key record returned by the relayer API key endpoint.
pub struct RelayerApiKey {
	api_key_id: RelayerApiKeyId,
	address: Address,
	created_at: OffsetDateTime,
	updated_at: OffsetDateTime,
}

type HmacSha256 = Hmac<Sha256>;

const QUERY_ADDRESS: &str = "address";
const QUERY_ID: &str = "id";
const QUERY_TYPE: &str = "type";
const CONTENT_TYPE_JSON: &str = "application/json";
const PATH_DEPLOYED: &[&str] = &["deployed"];
const PATH_NONCE: &[&str] = &["nonce"];
const PATH_RELAY_PAYLOAD: &[&str] = &["relay-payload"];
const PATH_RELAYER_API_KEYS: &[&str] = &["relayer", "api", "keys"];
const PATH_SUBMIT: &[&str] = &["submit"];
const PATH_TRANSACTION: &[&str] = &["transaction"];
const PATH_TRANSACTIONS: &[&str] = &["transactions"];
const SCHEME_HTTP: &str = "http";
const SCHEME_HTTPS: &str = "https";
const REDACTED: &str = "[REDACTED]";
const BUILDER_METHOD_POST: &str = "POST";
const BUILDER_HEADERS_BODY_EMPTY: &str = "";

impl RelayerTransactionState {
	const CONFIRMED_STATE: &str = "STATE_CONFIRMED";
	const EXECUTED_STATE: &str = "STATE_EXECUTED";
	const FAILED_STATE: &str = "STATE_FAILED";
	const INVALID_STATE: &str = "STATE_INVALID";
	const MINED_STATE: &str = "STATE_MINED";
	const NEW_STATE: &str = "STATE_NEW";

	/// Returns the canonical wire-format state string.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::New => Self::NEW_STATE,
			Self::Executed => Self::EXECUTED_STATE,
			Self::Mined => Self::MINED_STATE,
			Self::Confirmed => Self::CONFIRMED_STATE,
			Self::Failed => Self::FAILED_STATE,
			Self::Invalid => Self::INVALID_STATE,
		}
	}

	/// Returns whether the state is terminal according to the relayer documentation.
	pub fn is_terminal(&self) -> bool {
		matches!(self, Self::Confirmed | Self::Failed | Self::Invalid)
	}
}

impl RelayerTransactionKind {
	const PROXY_KIND: &str = "PROXY";
	const SAFE_CREATE_KIND: &str = "SAFE-CREATE";
	const SAFE_KIND: &str = "SAFE";

	/// Returns the canonical wire-format transaction kind string.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Safe => Self::SAFE_KIND,
			Self::SafeCreate => Self::SAFE_CREATE_KIND,
			Self::Proxy => Self::PROXY_KIND,
		}
	}
}

impl TransactionId {
	/// Returns the underlying UUID.
	pub fn raw(&self) -> Uuid {
		self.0
	}
}

impl RelayerApiKeyId {
	/// Returns the underlying UUID.
	pub fn raw(&self) -> Uuid {
		self.0
	}
}

impl TransactionMetadata {
	/// Creates validated transaction metadata.
	pub fn new(value: Cow<'static, str>) -> Result<Self, PolyrelError> {
		if value.is_empty() {
			return Err(PolyrelError::deserialize(
				"transaction metadata must not be empty",
			));
		}

		Ok(Self(value))
	}

	/// Returns the metadata as a string slice.
	pub fn as_str(&self) -> &str {
		self.0.as_ref()
	}
}

impl CurrentNonce {
	/// Returns the nonce value.
	pub fn nonce(&self) -> U256 {
		self.nonce
	}
}

impl RelayPayload {
	/// Returns the relayer address.
	pub fn address(&self) -> Address {
		self.address
	}

	/// Returns the relay nonce.
	pub fn nonce(&self) -> U256 {
		self.nonce
	}
}

impl SubmittedTransaction {
	/// Returns the relayer transaction identifier.
	pub fn transaction_id(&self) -> TransactionId {
		self.transaction_id
	}

	/// Returns the current relayer state.
	pub fn state(&self) -> RelayerTransactionState {
		self.state
	}

	/// Returns the optional hash field returned by the relayer.
	pub fn hash(&self) -> Option<B256> {
		self.hash
	}

	/// Returns the optional onchain transaction hash.
	pub fn transaction_hash(&self) -> Option<B256> {
		self.transaction_hash
	}
}

impl RelayerTransaction {
	/// Returns the relayer transaction identifier.
	pub fn transaction_id(&self) -> TransactionId {
		self.transaction_id
	}

	/// Returns the onchain transaction hash, if present.
	pub fn transaction_hash(&self) -> Option<B256> {
		self.transaction_hash
	}

	/// Returns the request sender address, if present.
	pub fn from(&self) -> Option<Address> {
		self.from
	}

	/// Returns the target address, if present.
	pub fn to(&self) -> Option<Address> {
		self.to
	}

	/// Returns the Safe or proxy wallet address, if present.
	pub fn proxy_address(&self) -> Option<Address> {
		self.proxy_address
	}

	/// Returns the raw calldata, if present.
	pub fn data(&self) -> Option<&Bytes> {
		self.data.as_ref()
	}

	/// Returns the transaction nonce, if present.
	pub fn nonce(&self) -> Option<U256> {
		self.nonce
	}

	/// Returns the native value, if present.
	pub fn value(&self) -> Option<U256> {
		self.value
	}

	/// Returns the current relayer state.
	pub fn state(&self) -> RelayerTransactionState {
		self.state
	}

	/// Returns the relayer transaction kind, if present.
	pub fn transaction_kind(&self) -> Option<RelayerTransactionKind> {
		self.transaction_kind
	}

	/// Returns relayer metadata, if present.
	pub fn metadata(&self) -> Option<&TransactionMetadata> {
		self.metadata.as_ref()
	}

	/// Returns the submitted signature bytes, if present.
	pub fn signature(&self) -> Option<&Bytes> {
		self.signature.as_ref()
	}

	/// Returns the owner address, if present.
	pub fn owner(&self) -> Option<Address> {
		self.owner
	}

	/// Returns the creation timestamp, if present.
	pub fn created_at(&self) -> Option<&OffsetDateTime> {
		self.created_at.as_ref()
	}

	/// Returns the last update timestamp, if present.
	pub fn updated_at(&self) -> Option<&OffsetDateTime> {
		self.updated_at.as_ref()
	}
}

impl RelayerApiKey {
	/// Returns the relayer API key identifier.
	pub fn api_key_id(&self) -> RelayerApiKeyId {
		self.api_key_id
	}

	/// Returns the address associated with the API key.
	pub fn address(&self) -> Address {
		self.address
	}

	/// Returns the creation timestamp.
	pub fn created_at(&self) -> &OffsetDateTime {
		&self.created_at
	}

	/// Returns the last update timestamp.
	pub fn updated_at(&self) -> &OffsetDateTime {
		&self.updated_at
	}
}

impl WalletQueryKind {
	const SAFE_KIND: &str = "SAFE";
	const PROXY_KIND: &str = "PROXY";

	/// Returns the wire-format query value expected by the relayer.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Safe => Self::SAFE_KIND,
			Self::Proxy => Self::PROXY_KIND,
		}
	}
}

impl RelayerApiKeyAuth {
	/// Header name for the relayer API key.
	pub const HEADER_API_KEY: &str = "RELAYER_API_KEY";
	/// Header name for the relayer API key address.
	pub const HEADER_API_KEY_ADDRESS: &str = "RELAYER_API_KEY_ADDRESS";
	const HEADER_API_KEY_LOWER: &str = "relayer_api_key";
	const HEADER_API_KEY_ADDRESS_LOWER: &str = "relayer_api_key_address";

	/// Creates relayer API key authentication material.
	pub fn new(key: SecretString, address: Address) -> Self {
		Self { key, address }
	}

	/// Returns the secret relayer API key.
	pub fn key(&self) -> &SecretString {
		&self.key
	}

	/// Returns the address associated with the API key.
	pub fn address(&self) -> Address {
		self.address
	}
}

impl BuilderAuth {
	/// Header name for the builder API key.
	pub const HEADER_API_KEY: &str = "POLY_BUILDER_API_KEY";
	/// Header name for the builder passphrase.
	pub const HEADER_PASSPHRASE: &str = "POLY_BUILDER_PASSPHRASE";
	/// Header name for the HMAC signature.
	pub const HEADER_SIGNATURE: &str = "POLY_BUILDER_SIGNATURE";
	/// Header name for the signature timestamp.
	pub const HEADER_TIMESTAMP: &str = "POLY_BUILDER_TIMESTAMP";
	const HEADER_API_KEY_LOWER: &str = "poly_builder_api_key";
	const HEADER_PASSPHRASE_LOWER: &str = "poly_builder_passphrase";
	const HEADER_SIGNATURE_LOWER: &str = "poly_builder_signature";
	const HEADER_TIMESTAMP_LOWER: &str = "poly_builder_timestamp";

	/// Creates builder HMAC authentication material.
	pub fn new(key: SecretString, secret: SecretString, passphrase: SecretString) -> Self {
		Self { key, secret, passphrase }
	}

	/// Returns the builder API key.
	pub fn key(&self) -> &SecretString {
		&self.key
	}

	/// Returns the builder secret used for HMAC signing.
	pub fn secret(&self) -> &SecretString {
		&self.secret
	}

	/// Returns the builder passphrase.
	pub fn passphrase(&self) -> &SecretString {
		&self.passphrase
	}
}

impl RelayerBaseUrl {
	/// Creates a validated relayer base URL from an already parsed [`Url`].
	pub fn new(mut url: Url) -> Result<Self, PolyrelError> {
		match url.scheme() {
			SCHEME_HTTP | SCHEME_HTTPS => {},
			_ => {
				return Err(PolyrelError::validation(
					"relayer base url must use http or https",
				));
			},
		}

		if url.query().is_some() {
			return Err(PolyrelError::validation(
				"relayer base url must not contain a query string",
			));
		}

		if url.fragment().is_some() {
			return Err(PolyrelError::validation(
				"relayer base url must not contain a fragment",
			));
		}

		let trimmed = url.path().trim_end_matches('/').to_owned();
		url.set_path(&trimmed);

		Ok(Self { url })
	}

	/// Parses and validates a relayer base URL.
	pub fn parse<U>(url: U) -> Result<Self, PolyrelError>
	where
		U: AsRef<str>,
	{
		let url = Url::parse(url.as_ref()).map_err(|e| PolyrelError::validation(e.to_string()))?;

		Self::new(url)
	}

	/// Returns the validated URL.
	pub fn as_url(&self) -> &Url {
		&self.url
	}
}

impl RelayerClient<Unauthenticated> {
	/// Creates a new unauthenticated relayer client with a default HTTP client.
	pub fn new(base_url: RelayerBaseUrl) -> Self {
		Self { base_url, http: reqwest::Client::new(), state: Unauthenticated }
	}

	/// Creates a new unauthenticated relayer client with a caller-supplied HTTP client.
	pub fn with_http(base_url: RelayerBaseUrl, http: reqwest::Client) -> Self {
		Self { base_url, http, state: Unauthenticated }
	}

	/// Authenticates the client with relayer API key headers.
	pub fn authenticate(self, auth: RelayerApiKeyAuth) -> RelayerClient<RelayerAuthenticated> {
		self.authenticate_relayer(auth)
	}

	/// Authenticates the client with relayer API key headers.
	pub fn authenticate_relayer(
		self,
		auth: RelayerApiKeyAuth,
	) -> RelayerClient<RelayerAuthenticated> {
		RelayerClient {
			base_url: self.base_url,
			http: self.http,
			state: RelayerAuthenticated { auth },
		}
	}

	/// Authenticates the client with builder HMAC headers.
	pub fn authenticate_builder(self, auth: BuilderAuth) -> RelayerClient<BuilderAuthenticated> {
		RelayerClient {
			base_url: self.base_url,
			http: self.http,
			state: BuilderAuthenticated { auth },
		}
	}
}

impl<S> RelayerClient<S> {
	/// Returns the validated relayer base URL.
	pub fn base_url(&self) -> &RelayerBaseUrl {
		&self.base_url
	}

	/// Fetches relayer transactions by relayer transaction identifier.
	pub async fn transaction_by_id(
		&self,
		transaction_id: &str,
	) -> Result<Vec<RelayerTransaction>, PolyrelError> {
		let url = self.endpoint(PATH_TRANSACTION)?;
		let response = self
			.http
			.get(url)
			.query(&[(QUERY_ID, transaction_id)])
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		let transactions: Vec<dto::RelayerTransaction> = handle_response(response).await?;

		transactions.into_iter().map(RelayerTransaction::try_from).collect()
	}

	/// Fetches the current nonce for a Safe or proxy wallet.
	pub async fn current_nonce(
		&self,
		address: Address,
		kind: WalletQueryKind,
	) -> Result<CurrentNonce, PolyrelError> {
		let url = self.endpoint(PATH_NONCE)?;
		let response = self
			.http
			.get(url)
			.query(&[
				(QUERY_ADDRESS, address_string(address)),
				(QUERY_TYPE, kind.as_str().to_owned()),
			])
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		let nonce: dto::NonceResponse = handle_response(response).await?;

		CurrentNonce::try_from(nonce)
	}

	/// Fetches the relay payload for proxy-based transactions.
	pub async fn relay_payload(
		&self,
		address: Address,
		kind: WalletQueryKind,
	) -> Result<RelayPayload, PolyrelError> {
		let url = self.endpoint(PATH_RELAY_PAYLOAD)?;
		let response = self
			.http
			.get(url)
			.query(&[
				(QUERY_ADDRESS, address_string(address)),
				(QUERY_TYPE, kind.as_str().to_owned()),
			])
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		let payload: dto::RelayPayloadResponse = handle_response(response).await?;

		RelayPayload::try_from(payload)
	}

	/// Returns whether a Safe wallet has already been deployed.
	pub async fn is_safe_deployed(&self, address: Address) -> Result<bool, PolyrelError> {
		#[derive(Deserialize)]
		struct DeployedResponse {
			deployed: bool,
		}

		let url = self.endpoint(PATH_DEPLOYED)?;
		let response = self
			.http
			.get(url)
			.query(&[(QUERY_ADDRESS, address_string(address))])
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		let deployed: DeployedResponse = handle_response(response).await?;
		Ok(deployed.deployed)
	}

	fn endpoint(&self, segments: &[&str]) -> Result<Url, PolyrelError> {
		let mut url = self.base_url.as_url().clone();
		{
			let mut path = url.path_segments_mut().map_err(|_| {
				PolyrelError::validation("relayer base url does not support path segments")
			})?;

			for segment in segments {
				path.push(segment);
			}
		}

		Ok(url)
	}
}

impl<S> RelayerClient<S>
where
	S: sealed::Authenticated,
{
	/// Submits a signed relayer request.
	pub async fn submit(
		&self,
		request: &SubmitRequest,
	) -> Result<SubmittedTransaction, PolyrelError> {
		let url = self.endpoint(PATH_SUBMIT)?;
		let body =
			serde_json::to_string(request).map_err(|e| PolyrelError::serialize(e.to_string()))?;
		let response = self
			.http
			.post(url.clone())
			.headers(authenticated_headers(
				&self.state,
				BUILDER_METHOD_POST,
				url.path(),
				&body,
			)?)
			.header(CONTENT_TYPE, CONTENT_TYPE_JSON)
			.body(body)
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		let submitted: dto::SubmitResponse = handle_response(response).await?;

		SubmittedTransaction::try_from(submitted)
	}

	/// Returns recent relayer transactions for the authenticated principal.
	pub async fn recent_transactions(&self) -> Result<Vec<RelayerTransaction>, PolyrelError> {
		let url = self.endpoint(PATH_TRANSACTIONS)?;
		let response = self
			.http
			.get(url.clone())
			.headers(authenticated_headers(
				&self.state,
				reqwest::Method::GET.as_str(),
				url.path(),
				BUILDER_HEADERS_BODY_EMPTY,
			)?)
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		let transactions: Vec<dto::RelayerTransaction> = handle_response(response).await?;

		transactions.into_iter().map(RelayerTransaction::try_from).collect()
	}
}

impl RelayerClient<RelayerAuthenticated> {
	/// Returns the relayer API key auth material associated with the client.
	pub fn auth(&self) -> &RelayerApiKeyAuth {
		&self.state.auth
	}

	/// Lists relayer API keys for the authenticated relayer principal.
	pub async fn relayer_api_keys(&self) -> Result<Vec<RelayerApiKey>, PolyrelError> {
		let url = self.endpoint(PATH_RELAYER_API_KEYS)?;
		let response = self
			.http
			.get(url.clone())
			.headers(authenticated_headers(
				&self.state,
				reqwest::Method::GET.as_str(),
				url.path(),
				BUILDER_HEADERS_BODY_EMPTY,
			)?)
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		let api_keys: Vec<dto::RelayerApiKeyRecord> = handle_response(response).await?;

		api_keys.into_iter().map(RelayerApiKey::try_from).collect()
	}
}

impl RelayerClient<BuilderAuthenticated> {
	/// Returns the builder auth material associated with the client.
	pub fn auth(&self) -> &BuilderAuth {
		&self.state.auth
	}
}

impl core::fmt::Debug for RelayerApiKeyAuth {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("RelayerApiKeyAuth")
			.field("key", &REDACTED)
			.field("address", &self.address)
			.finish()
	}
}

impl core::fmt::Debug for BuilderAuth {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("BuilderAuth")
			.field("key", &REDACTED)
			.field("secret", &REDACTED)
			.field("passphrase", &REDACTED)
			.finish()
	}
}

impl FromStr for RelayerTransactionState {
	type Err = PolyrelError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			Self::NEW_STATE => Ok(Self::New),
			Self::EXECUTED_STATE => Ok(Self::Executed),
			Self::MINED_STATE => Ok(Self::Mined),
			Self::CONFIRMED_STATE => Ok(Self::Confirmed),
			Self::FAILED_STATE => Ok(Self::Failed),
			Self::INVALID_STATE => Ok(Self::Invalid),
			_ => Err(PolyrelError::deserialize(format!(
				"unknown transaction state: {s}",
			))),
		}
	}
}

impl FromStr for RelayerTransactionKind {
	type Err = PolyrelError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			Self::SAFE_KIND => Ok(Self::Safe),
			Self::SAFE_CREATE_KIND => Ok(Self::SafeCreate),
			Self::PROXY_KIND => Ok(Self::Proxy),
			_ => Err(PolyrelError::deserialize(format!(
				"unknown transaction kind: {s}",
			))),
		}
	}
}

impl TryFrom<dto::NonceResponse> for CurrentNonce {
	type Error = PolyrelError;

	fn try_from(value: dto::NonceResponse) -> Result<Self, Self::Error> {
		Ok(Self { nonce: parse_required_u256(&value.nonce, "nonce")? })
	}
}

impl TryFrom<dto::RelayPayloadResponse> for RelayPayload {
	type Error = PolyrelError;

	fn try_from(value: dto::RelayPayloadResponse) -> Result<Self, Self::Error> {
		Ok(Self {
			address: parse_required_address(&value.address, "address")?,
			nonce: parse_required_u256(&value.nonce, "nonce")?,
		})
	}
}

impl TryFrom<dto::SubmitResponse> for SubmittedTransaction {
	type Error = PolyrelError;

	fn try_from(value: dto::SubmitResponse) -> Result<Self, Self::Error> {
		Ok(Self {
			transaction_id: TransactionId(parse_required_uuid(
				&value.transaction_id,
				"transactionID",
			)?),
			state: parse_required_state(&value.state)?,
			hash: parse_optional_b256(value.hash, "hash")?,
			transaction_hash: parse_optional_b256(value.transaction_hash, "transactionHash")?,
		})
	}
}

impl TryFrom<dto::RelayerTransaction> for RelayerTransaction {
	type Error = PolyrelError;

	fn try_from(value: dto::RelayerTransaction) -> Result<Self, Self::Error> {
		Ok(Self {
			transaction_id: TransactionId(parse_required_uuid(
				&value.transaction_id,
				"transactionID",
			)?),
			transaction_hash: parse_optional_b256(value.transaction_hash, "transactionHash")?,
			from: parse_optional_address(value.from, "from")?,
			to: parse_optional_address(value.to, "to")?,
			proxy_address: parse_optional_address(value.proxy_address, "proxyAddress")?,
			data: parse_optional_bytes(value.data, "data")?,
			nonce: parse_optional_u256(value.nonce, "nonce")?,
			value: parse_optional_u256(value.value, "value")?,
			state: parse_required_state(&value.state)?,
			transaction_kind: parse_optional_transaction_kind(value.transaction_type)?,
			metadata: parse_optional_metadata(value.metadata)?,
			signature: parse_optional_bytes(value.signature, "signature")?,
			owner: parse_optional_address(value.owner, "owner")?,
			created_at: parse_optional_timestamp(value.created_at, "createdAt")?,
			updated_at: parse_optional_timestamp(value.updated_at, "updatedAt")?,
		})
	}
}

impl TryFrom<dto::RelayerApiKeyRecord> for RelayerApiKey {
	type Error = PolyrelError;

	fn try_from(value: dto::RelayerApiKeyRecord) -> Result<Self, Self::Error> {
		Ok(Self {
			api_key_id: RelayerApiKeyId(parse_required_uuid(&value.api_key, "apiKey")?),
			address: parse_required_address(&value.address, "address")?,
			created_at: parse_required_timestamp(&value.created_at, "createdAt")?,
			updated_at: parse_required_timestamp(&value.updated_at, "updatedAt")?,
		})
	}
}

impl sealed::Authenticated for RelayerAuthenticated {
	fn headers(&self, _method: &str, _path: &str, _body: &str) -> Result<HeaderMap, PolyrelError> {
		relayer_headers(&self.auth)
	}
}

impl sealed::Authenticated for BuilderAuthenticated {
	fn headers(&self, method: &str, path: &str, body: &str) -> Result<HeaderMap, PolyrelError> {
		builder_headers(&self.auth, method, path, body)
	}
}

fn relayer_headers(auth: &RelayerApiKeyAuth) -> Result<HeaderMap, PolyrelError> {
	let mut headers = HeaderMap::new();
	let api_key_header = HeaderName::from_static(RelayerApiKeyAuth::HEADER_API_KEY_LOWER);
	let api_key_address_header =
		HeaderName::from_static(RelayerApiKeyAuth::HEADER_API_KEY_ADDRESS_LOWER);
	let api_key = HeaderValue::from_str(auth.key().expose_secret())
		.map_err(|e| PolyrelError::validation(e.to_string()))?;
	let api_key_address = HeaderValue::from_str(&format!("{:#x}", auth.address()))
		.map_err(|e| PolyrelError::validation(e.to_string()))?;

	headers.insert(api_key_header, api_key);
	headers.insert(api_key_address_header, api_key_address);

	Ok(headers)
}

fn builder_headers(
	auth: &BuilderAuth,
	method: &str,
	path: &str,
	body: &str,
) -> Result<HeaderMap, PolyrelError> {
	let timestamp = timestamp_seconds()?;

	builder_headers_with_timestamp(auth, method, path, body, timestamp)
}

fn builder_headers_with_timestamp(
	auth: &BuilderAuth,
	method: &str,
	path: &str,
	body: &str,
	timestamp: u64,
) -> Result<HeaderMap, PolyrelError> {
	let decoded_secret = decode_builder_secret(auth.secret().expose_secret())
		.map_err(|e| PolyrelError::validation(e.to_string()))?;
	let message = format!("{timestamp}{method}{path}{body}");
	let mut mac = HmacSha256::new_from_slice(&decoded_secret)
		.map_err(|e| PolyrelError::validation(e.to_string()))?;
	mac.update(message.as_bytes());

	let mut headers = HeaderMap::new();
	headers.insert(
		HeaderName::from_static(BuilderAuth::HEADER_API_KEY_LOWER),
		HeaderValue::from_str(auth.key().expose_secret())
			.map_err(|e| PolyrelError::validation(e.to_string()))?,
	);
	headers.insert(
		HeaderName::from_static(BuilderAuth::HEADER_PASSPHRASE_LOWER),
		HeaderValue::from_str(auth.passphrase().expose_secret())
			.map_err(|e| PolyrelError::validation(e.to_string()))?,
	);
	headers.insert(
		HeaderName::from_static(BuilderAuth::HEADER_SIGNATURE_LOWER),
		HeaderValue::from_str(&builder_signature(mac.finalize().into_bytes().as_ref()))
			.map_err(|e| PolyrelError::validation(e.to_string()))?,
	);
	headers.insert(
		HeaderName::from_static(BuilderAuth::HEADER_TIMESTAMP_LOWER),
		HeaderValue::from_str(&timestamp.to_string())
			.map_err(|e| PolyrelError::validation(e.to_string()))?,
	);

	Ok(headers)
}

fn decode_builder_secret(secret: &str) -> Result<Vec<u8>, base64::DecodeError> {
	STANDARD
		.decode(secret.as_bytes())
		.or_else(|_| STANDARD_NO_PAD.decode(secret.as_bytes()))
		.or_else(|_| URL_SAFE.decode(secret.as_bytes()))
		.or_else(|_| URL_SAFE_NO_PAD.decode(secret.as_bytes()))
}

fn builder_signature(raw: &[u8]) -> String {
	STANDARD.encode(raw).replace('+', "-").replace('/', "_")
}

fn authenticated_headers<S>(
	state: &S,
	method: &str,
	path: &str,
	body: &str,
) -> Result<HeaderMap, PolyrelError>
where
	S: sealed::Authenticated,
{
	<S as sealed::Authenticated>::headers(state, method, path, body)
}

fn timestamp_seconds() -> Result<u64, PolyrelError> {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|duration| duration.as_secs())
		.map_err(|e| PolyrelError::validation(e.to_string()))
}

fn parse_required_state(value: &str) -> Result<RelayerTransactionState, PolyrelError> {
	required_non_empty(value, "state")?.parse()
}

fn parse_required_address(value: &str, field: &str) -> Result<Address, PolyrelError> {
	required_non_empty(value, field)?
		.parse()
		.map_err(|e| PolyrelError::deserialize(format!("invalid {field}: {e}")))
}

fn parse_optional_address(
	value: Option<String>,
	field: &str,
) -> Result<Option<Address>, PolyrelError> {
	normalize_optional_string(value).map(|value| parse_required_address(&value, field)).transpose()
}

fn parse_optional_b256(value: Option<String>, field: &str) -> Result<Option<B256>, PolyrelError> {
	normalize_optional_string(value)
		.map(|value| {
			B256::from_str(&value)
				.map_err(|e| PolyrelError::deserialize(format!("invalid {field}: {e}")))
		})
		.transpose()
}

fn parse_optional_bytes(value: Option<String>, field: &str) -> Result<Option<Bytes>, PolyrelError> {
	normalize_optional_string(value).map(|value| parse_required_bytes(&value, field)).transpose()
}

fn parse_required_bytes(value: &str, field: &str) -> Result<Bytes, PolyrelError> {
	let raw = required_non_empty(value, field)?.strip_prefix("0x").unwrap_or(value);
	let decoded = alloy_primitives::hex::decode(raw)
		.map_err(|e| PolyrelError::deserialize(format!("invalid {field}: {e}")))?;

	Ok(Bytes::from(decoded))
}

fn parse_required_timestamp(value: &str, field: &str) -> Result<OffsetDateTime, PolyrelError> {
	OffsetDateTime::parse(required_non_empty(value, field)?, &Rfc3339)
		.map_err(|e| PolyrelError::deserialize(format!("invalid {field}: {e}")))
}

fn parse_optional_timestamp(
	value: Option<String>,
	field: &str,
) -> Result<Option<OffsetDateTime>, PolyrelError> {
	normalize_optional_string(value)
		.map(|value| parse_required_timestamp(&value, field))
		.transpose()
}

fn parse_required_u256(value: &str, field: &str) -> Result<U256, PolyrelError> {
	U256::from_str(required_non_empty(value, field)?)
		.map_err(|e| PolyrelError::deserialize(format!("invalid {field}: {e}")))
}

fn parse_optional_u256(value: Option<String>, field: &str) -> Result<Option<U256>, PolyrelError> {
	normalize_optional_string(value).map(|value| parse_required_u256(&value, field)).transpose()
}

fn parse_required_uuid(value: &str, field: &str) -> Result<Uuid, PolyrelError> {
	Uuid::parse_str(required_non_empty(value, field)?)
		.map_err(|e| PolyrelError::deserialize(format!("invalid {field}: {e}")))
}

fn parse_optional_transaction_kind(
	value: Option<String>,
) -> Result<Option<RelayerTransactionKind>, PolyrelError> {
	normalize_optional_string(value).map(|value| value.parse()).transpose()
}

fn parse_optional_metadata(
	value: Option<String>,
) -> Result<Option<TransactionMetadata>, PolyrelError> {
	normalize_optional_string(value)
		.map(|value| TransactionMetadata::new(Cow::Owned(value)))
		.transpose()
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
	value.filter(|value| !value.is_empty())
}

fn required_non_empty<'a>(value: &'a str, field: &str) -> Result<&'a str, PolyrelError> {
	if value.is_empty() {
		return Err(PolyrelError::deserialize(format!(
			"{field} must not be empty",
		)));
	}

	Ok(value)
}

fn address_string(address: Address) -> String {
	format!("{address:#x}")
}

async fn handle_response<T>(response: reqwest::Response) -> Result<T, PolyrelError>
where
	T: serde::de::DeserializeOwned,
{
	let status = response.status();
	if !status.is_success() {
		let body = response.text().await.map_err(|e| PolyrelError::http(e.to_string()))?;

		return Err(PolyrelError::Api { status: status.as_u16(), body: Cow::Owned(body) });
	}

	response.json::<T>().await.map_err(|e| PolyrelError::deserialize(e.to_string()))
}

#[cfg(test)]
mod tests {
	use alloc::borrow::Cow;

	use alloy_primitives::{B256, Signature, U256, address, b256};
	use reqwest::Method;
	use secrecy::SecretString;
	use time::{OffsetDateTime, format_description::well_known::Rfc3339};
	use uuid::Uuid;
	use wiremock::{
		Mock, MockServer, ResponseTemplate,
		matchers::{body_json, header, header_exists, method, path, query_param},
	};

	use super::*;
	use crate::safe::{
		ChainId, FactoryDomainName, SafeCreateContext, SafeCreatePayment, build_create_draft,
	};

	const AUTH_ADDRESS: Address = address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5");
	const SAFE_ADDRESS: Address = address!("6d8c4e9adf5748af82dabe2c6225207770d6b4fa");
	const SAFE_FACTORY: Address = address!("aacfeea03eb1561c4e67d661e40682bd20e3541b");
	const SAFE_INIT_CODE_HASH: B256 =
		b256!("2bce2127ff07fb632d16c8347c4ebf501f4841168bed00d9e6ef715ddb6fcecf");
	const FACTORY_DOMAIN_NAME: &str = "Polymarket Contract Proxy Factory";
	const AUTH_KEY: &str = "secret-key";
	const NONCE_VALUE: &str = "12";
	const RELAY_PAYLOAD_ADDRESS: &str = "0x1111111111111111111111111111111111111111";
	const RELAY_PAYLOAD_NONCE: &str = "9";
	const TRANSACTION_ID: &str = "01967c03-b8c8-7000-8f68-8b8eaec6fd3d";
	const SUBMIT_TRANSACTION_ID: &str = "01967c03-b8c8-7000-8f68-8b8eaec6fd3e";
	const RECENT_TRANSACTION_ID: &str = "01967c03-b8c8-7000-8f68-8b8eaec6fd3f";
	const API_KEY_RECORD: &str = "01967c03-b8c8-7000-8f68-8b8eaec6fd3d";
	const API_KEY_TIMESTAMP: &str = "2026-02-24T18:20:11.237485Z";
	const MOCK_STATE_NEW: &str = "STATE_NEW";
	const BUILDER_KEY: &str = "builder-key";
	const BUILDER_SECRET: &str = "dGVzdC1zZWNyZXQ=";
	const BUILDER_PASSPHRASE: &str = "builder-pass";
	const BUILDER_TIMESTAMP: u64 = 1_710_000_000;
	const BUILDER_PATH: &str = "/submit";
	const BUILDER_BODY: &str = "{\"x\":1}";
	const BUILDER_EXPECTED_SIGNATURE: &str = "NLXH0LVQnqYXcPQJHppZ1dS1TUMqkGFrmfMrIcYCTJY=";

	fn base_url(server: &MockServer) -> RelayerBaseUrl {
		RelayerBaseUrl::parse(Cow::Owned(server.uri())).unwrap()
	}

	fn relayer_auth() -> RelayerApiKeyAuth {
		RelayerApiKeyAuth::new(SecretString::from(AUTH_KEY), AUTH_ADDRESS)
	}

	fn builder_auth() -> BuilderAuth {
		BuilderAuth::new(
			SecretString::from(BUILDER_KEY),
			SecretString::from(BUILDER_SECRET),
			SecretString::from(BUILDER_PASSPHRASE),
		)
	}

	fn relayer_transaction_dto() -> dto::RelayerTransaction {
		dto::RelayerTransaction {
			transaction_id: TRANSACTION_ID.to_owned(),
			transaction_hash: Some(
				"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
			),
			from: Some(format!("{AUTH_ADDRESS:#x}")),
			to: Some(format!("{SAFE_FACTORY:#x}")),
			proxy_address: Some(format!("{SAFE_ADDRESS:#x}")),
			data: Some("0xdeadbeef".to_owned()),
			nonce: Some("12".to_owned()),
			value: Some("34".to_owned()),
			state: RelayerTransactionState::CONFIRMED_STATE.to_owned(),
			transaction_type: Some(RelayerTransactionKind::SAFE_CREATE_KIND.to_owned()),
			metadata: Some("builder metadata".to_owned()),
			signature: Some("0x1234".to_owned()),
			owner: Some(format!("{AUTH_ADDRESS:#x}")),
			created_at: Some(API_KEY_TIMESTAMP.to_owned()),
			updated_at: Some(API_KEY_TIMESTAMP.to_owned()),
		}
	}

	fn submit_response_dto() -> dto::SubmitResponse {
		dto::SubmitResponse {
			transaction_id: SUBMIT_TRANSACTION_ID.to_owned(),
			state: RelayerTransactionState::MINED_STATE.to_owned(),
			hash: Some(
				"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
			),
			transaction_hash: Some(
				"0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned(),
			),
		}
	}

	fn relayer_api_key_dto() -> dto::RelayerApiKeyRecord {
		dto::RelayerApiKeyRecord {
			api_key: API_KEY_RECORD.to_owned(),
			address: format!("{AUTH_ADDRESS:#x}"),
			created_at: API_KEY_TIMESTAMP.to_owned(),
			updated_at: API_KEY_TIMESTAMP.to_owned(),
		}
	}

	#[test]
	fn builder_auth_debug_redacts_secrets() {
		// Arrange
		let auth = builder_auth();

		// Act
		let debug = format!("{auth:?}");

		// Assert
		assert!(!debug.contains(BUILDER_KEY));
		assert!(!debug.contains(BUILDER_SECRET));
		assert!(!debug.contains(BUILDER_PASSPHRASE));
		assert!(debug.contains(REDACTED));
	}

	#[test]
	fn builder_headers_are_deterministic_for_fixed_inputs() {
		// Arrange
		let auth = builder_auth();

		// Act
		let headers = builder_headers_with_timestamp(
			&auth,
			BUILDER_METHOD_POST,
			BUILDER_PATH,
			BUILDER_BODY,
			BUILDER_TIMESTAMP,
		)
		.unwrap();

		// Assert
		assert_eq!(
			headers.get(BuilderAuth::HEADER_API_KEY_LOWER).unwrap().to_str().unwrap(),
			BUILDER_KEY
		);
		assert_eq!(
			headers.get(BuilderAuth::HEADER_PASSPHRASE_LOWER).unwrap().to_str().unwrap(),
			BUILDER_PASSPHRASE
		);
		assert_eq!(
			headers.get(BuilderAuth::HEADER_SIGNATURE_LOWER).unwrap().to_str().unwrap(),
			BUILDER_EXPECTED_SIGNATURE
		);
		assert_eq!(
			headers.get(BuilderAuth::HEADER_TIMESTAMP_LOWER).unwrap().to_str().unwrap(),
			&BUILDER_TIMESTAMP.to_string()
		);
	}

	#[test]
	fn relayer_api_key_debug_redacts_key() {
		// Arrange
		let auth = relayer_auth();

		// Act
		let debug = format!("{auth:?}");

		// Assert
		assert!(!debug.contains(AUTH_KEY));
		assert!(debug.contains(REDACTED));
	}

	#[tokio::test]
	async fn current_nonce_hits_expected_path_and_query() {
		// Arrange
		let server = MockServer::start().await;
		Mock::given(method(Method::GET.as_str()))
			.and(path("/nonce"))
			.and(query_param(QUERY_ADDRESS, format!("{AUTH_ADDRESS:#x}")))
			.and(query_param(QUERY_TYPE, WalletQueryKind::SAFE_KIND))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
				"nonce": NONCE_VALUE
			})))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server));

		// Act
		let response = client.current_nonce(AUTH_ADDRESS, WalletQueryKind::Safe).await.unwrap();

		// Assert
		assert_eq!(response.nonce(), U256::from(12_u64));
	}

	#[tokio::test]
	async fn relay_payload_hits_expected_path_and_query() {
		// Arrange
		let server = MockServer::start().await;
		Mock::given(method(Method::GET.as_str()))
			.and(path("/relay-payload"))
			.and(query_param(QUERY_ADDRESS, format!("{AUTH_ADDRESS:#x}")))
			.and(query_param(QUERY_TYPE, WalletQueryKind::PROXY_KIND))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
				"address": RELAY_PAYLOAD_ADDRESS,
				"nonce": RELAY_PAYLOAD_NONCE
			})))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server));

		// Act
		let response = client.relay_payload(AUTH_ADDRESS, WalletQueryKind::Proxy).await.unwrap();

		// Assert
		assert_eq!(
			response.address(),
			RELAY_PAYLOAD_ADDRESS.parse::<Address>().unwrap()
		);
		assert_eq!(response.nonce(), U256::from(9_u64));
	}

	#[tokio::test]
	async fn is_safe_deployed_hits_expected_path_and_query() {
		// Arrange
		let server = MockServer::start().await;
		Mock::given(method(Method::GET.as_str()))
			.and(path("/deployed"))
			.and(query_param(QUERY_ADDRESS, format!("{SAFE_ADDRESS:#x}")))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
				"deployed": true
			})))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server));

		// Act
		let deployed = client.is_safe_deployed(SAFE_ADDRESS).await.unwrap();

		// Assert
		assert!(deployed);
	}

	#[tokio::test]
	async fn transaction_by_id_hits_expected_path_and_query() {
		// Arrange
		let server = MockServer::start().await;
		Mock::given(method(Method::GET.as_str()))
			.and(path("/transaction"))
			.and(query_param(QUERY_ID, TRANSACTION_ID))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
				{
					"transactionID": TRANSACTION_ID,
					"state": MOCK_STATE_NEW
				}
			])))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server));

		// Act
		let response = client.transaction_by_id(TRANSACTION_ID).await.unwrap();

		// Assert
		assert_eq!(response.len(), 1);
		assert_eq!(
			response[0].transaction_id().raw(),
			Uuid::parse_str(TRANSACTION_ID).unwrap()
		);
		assert_eq!(response[0].state(), RelayerTransactionState::New);
	}

	#[tokio::test]
	async fn submit_sends_relayer_auth_headers_and_body() {
		// Arrange
		let server = MockServer::start().await;
		let request = build_create_draft(
			&SafeCreateContext::builder()
				.owner(AUTH_ADDRESS)
				.chain_id(ChainId::new(137.try_into().unwrap()))
				.safe_factory(SAFE_FACTORY)
				.safe_init_code_hash(SAFE_INIT_CODE_HASH)
				.factory_domain_name(FactoryDomainName::new(FACTORY_DOMAIN_NAME.into()).unwrap())
				.build(),
			&SafeCreatePayment::builder()
				.payment_token(Address::ZERO)
				.payment(U256::ZERO)
				.payment_receiver(Address::ZERO)
				.build(),
		)
		.into_submit_request({
			let mut bytes = [0xbb; 65];
			bytes[64] = 27;
			Signature::from_raw_array(&bytes).unwrap()
		});
		Mock::given(method(Method::POST.as_str()))
			.and(path("/submit"))
			.and(header(RelayerApiKeyAuth::HEADER_API_KEY, AUTH_KEY))
			.and(header(
				RelayerApiKeyAuth::HEADER_API_KEY_ADDRESS,
				&format!("{AUTH_ADDRESS:#x}"),
			))
			.and(body_json(&request))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
				"transactionID": SUBMIT_TRANSACTION_ID,
				"state": MOCK_STATE_NEW
			})))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server)).authenticate_relayer(relayer_auth());

		// Act
		let response = client.submit(&request).await.unwrap();

		// Assert
		assert_eq!(
			response.transaction_id().raw(),
			Uuid::parse_str(SUBMIT_TRANSACTION_ID).unwrap()
		);
		assert_eq!(response.state(), RelayerTransactionState::New);
	}

	#[tokio::test]
	async fn submit_sends_builder_auth_headers_and_body() {
		// Arrange
		let server = MockServer::start().await;
		let request = build_create_draft(
			&SafeCreateContext::builder()
				.owner(AUTH_ADDRESS)
				.chain_id(ChainId::new(137.try_into().unwrap()))
				.safe_factory(SAFE_FACTORY)
				.safe_init_code_hash(SAFE_INIT_CODE_HASH)
				.factory_domain_name(FactoryDomainName::new(FACTORY_DOMAIN_NAME.into()).unwrap())
				.build(),
			&SafeCreatePayment::builder()
				.payment_token(Address::ZERO)
				.payment(U256::ZERO)
				.payment_receiver(Address::ZERO)
				.build(),
		)
		.into_submit_request({
			let mut bytes = [0xbb; 65];
			bytes[64] = 27;
			Signature::from_raw_array(&bytes).unwrap()
		});
		Mock::given(method(Method::POST.as_str()))
			.and(path("/submit"))
			.and(header(BuilderAuth::HEADER_API_KEY, BUILDER_KEY))
			.and(header(BuilderAuth::HEADER_PASSPHRASE, BUILDER_PASSPHRASE))
			.and(header_exists(BuilderAuth::HEADER_TIMESTAMP))
			.and(header_exists(BuilderAuth::HEADER_SIGNATURE))
			.and(body_json(&request))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
				"transactionID": SUBMIT_TRANSACTION_ID,
				"state": MOCK_STATE_NEW
			})))
			.mount(&server)
			.await;
		let client = RelayerClient::with_http(base_url(&server), reqwest::Client::new())
			.authenticate_builder(builder_auth());

		// Act
		let response = client.submit(&request).await.unwrap();

		// Assert
		assert_eq!(
			response.transaction_id().raw(),
			Uuid::parse_str(SUBMIT_TRANSACTION_ID).unwrap()
		);
		assert_eq!(response.state(), RelayerTransactionState::New);
	}

	#[tokio::test]
	async fn recent_transactions_sends_relayer_auth_headers() {
		// Arrange
		let server = MockServer::start().await;
		Mock::given(method(Method::GET.as_str()))
			.and(path("/transactions"))
			.and(header(RelayerApiKeyAuth::HEADER_API_KEY, AUTH_KEY))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
				{
					"transactionID": RECENT_TRANSACTION_ID,
					"state": MOCK_STATE_NEW
				}
			])))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server)).authenticate_relayer(relayer_auth());

		// Act
		let response = client.recent_transactions().await.unwrap();

		// Assert
		assert_eq!(response.len(), 1);
		assert_eq!(
			response[0].transaction_id().raw(),
			Uuid::parse_str(RECENT_TRANSACTION_ID).unwrap()
		);
	}

	#[tokio::test]
	async fn recent_transactions_sends_builder_auth_headers() {
		// Arrange
		let server = MockServer::start().await;
		Mock::given(method(Method::GET.as_str()))
			.and(path("/transactions"))
			.and(header(BuilderAuth::HEADER_API_KEY, BUILDER_KEY))
			.and(header(BuilderAuth::HEADER_PASSPHRASE, BUILDER_PASSPHRASE))
			.and(header_exists(BuilderAuth::HEADER_TIMESTAMP))
			.and(header_exists(BuilderAuth::HEADER_SIGNATURE))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
				{
					"transactionID": RECENT_TRANSACTION_ID,
					"state": MOCK_STATE_NEW
				}
			])))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server)).authenticate_builder(builder_auth());

		// Act
		let response = client.recent_transactions().await.unwrap();

		// Assert
		assert_eq!(response.len(), 1);
		assert_eq!(
			response[0].transaction_id().raw(),
			Uuid::parse_str(RECENT_TRANSACTION_ID).unwrap()
		);
	}

	#[tokio::test]
	async fn relayer_api_keys_hits_expected_endpoint() {
		// Arrange
		let server = MockServer::start().await;
		Mock::given(method(Method::GET.as_str()))
			.and(path("/relayer/api/keys"))
			.and(header(RelayerApiKeyAuth::HEADER_API_KEY, AUTH_KEY))
			.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
				{
					"apiKey": API_KEY_RECORD,
					"address": format!("{AUTH_ADDRESS:#x}"),
					"createdAt": API_KEY_TIMESTAMP,
					"updatedAt": API_KEY_TIMESTAMP
				}
			])))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server)).authenticate_relayer(relayer_auth());

		// Act
		let response = client.relayer_api_keys().await.unwrap();

		// Assert
		assert_eq!(response.len(), 1);
		assert_eq!(
			response[0].api_key_id().raw(),
			Uuid::parse_str(API_KEY_RECORD).unwrap()
		);
		assert_eq!(response[0].address(), AUTH_ADDRESS);
		assert_eq!(
			*response[0].created_at(),
			OffsetDateTime::parse(API_KEY_TIMESTAMP, &Rfc3339).unwrap()
		);
	}

	#[test]
	fn relayer_transaction_state_matches_documented_variants() {
		// Arrange
		let terminal_states = [
			RelayerTransactionState::Confirmed,
			RelayerTransactionState::Failed,
			RelayerTransactionState::Invalid,
		];
		let non_terminal_states = [
			RelayerTransactionState::New,
			RelayerTransactionState::Executed,
			RelayerTransactionState::Mined,
		];

		// Act / Assert
		assert_eq!(
			RelayerTransactionState::from_str(RelayerTransactionState::NEW_STATE).unwrap(),
			RelayerTransactionState::New
		);
		assert_eq!(
			RelayerTransactionState::from_str(RelayerTransactionState::EXECUTED_STATE).unwrap(),
			RelayerTransactionState::Executed
		);
		assert_eq!(
			RelayerTransactionState::from_str(RelayerTransactionState::MINED_STATE).unwrap(),
			RelayerTransactionState::Mined
		);
		assert_eq!(
			RelayerTransactionState::from_str(RelayerTransactionState::CONFIRMED_STATE).unwrap(),
			RelayerTransactionState::Confirmed
		);
		assert_eq!(
			RelayerTransactionState::from_str(RelayerTransactionState::FAILED_STATE).unwrap(),
			RelayerTransactionState::Failed
		);
		assert_eq!(
			RelayerTransactionState::from_str(RelayerTransactionState::INVALID_STATE).unwrap(),
			RelayerTransactionState::Invalid
		);
		assert!(terminal_states.into_iter().all(|state| state.is_terminal()));
		assert!(non_terminal_states.into_iter().all(|state| !state.is_terminal()));
	}

	#[test]
	fn relayer_transaction_kind_matches_documented_variants() {
		// Act / Assert
		assert_eq!(
			RelayerTransactionKind::from_str(RelayerTransactionKind::SAFE_KIND).unwrap(),
			RelayerTransactionKind::Safe
		);
		assert_eq!(
			RelayerTransactionKind::from_str(RelayerTransactionKind::SAFE_CREATE_KIND).unwrap(),
			RelayerTransactionKind::SafeCreate
		);
		assert_eq!(
			RelayerTransactionKind::from_str(RelayerTransactionKind::PROXY_KIND).unwrap(),
			RelayerTransactionKind::Proxy
		);
	}

	#[test]
	fn relayer_transaction_converts_structured_fields() {
		// Arrange
		let dto = relayer_transaction_dto();

		// Act
		let transaction = RelayerTransaction::try_from(dto).unwrap();

		// Assert
		assert_eq!(
			transaction.transaction_id().raw(),
			Uuid::parse_str(TRANSACTION_ID).unwrap()
		);
		assert_eq!(
			transaction.transaction_hash(),
			Some(
				B256::from_str(
					"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
				)
				.unwrap()
			)
		);
		assert_eq!(transaction.from(), Some(AUTH_ADDRESS));
		assert_eq!(transaction.to(), Some(SAFE_FACTORY));
		assert_eq!(transaction.proxy_address(), Some(SAFE_ADDRESS));
		assert_eq!(
			transaction.data(),
			Some(&Bytes::from(vec![0xde, 0xad, 0xbe, 0xef]))
		);
		assert_eq!(transaction.nonce(), Some(U256::from(12_u64)));
		assert_eq!(transaction.value(), Some(U256::from(34_u64)));
		assert_eq!(transaction.state(), RelayerTransactionState::Confirmed);
		assert_eq!(
			transaction.transaction_kind(),
			Some(RelayerTransactionKind::SafeCreate)
		);
		assert_eq!(transaction.metadata().unwrap().as_str(), "builder metadata");
		assert_eq!(
			transaction.signature(),
			Some(&Bytes::from(vec![0x12, 0x34]))
		);
		assert_eq!(transaction.owner(), Some(AUTH_ADDRESS));
		assert_eq!(
			*transaction.created_at().unwrap(),
			OffsetDateTime::parse(API_KEY_TIMESTAMP, &Rfc3339).unwrap()
		);
	}

	#[test]
	fn relayer_transaction_normalizes_optional_empty_fields_to_none() {
		// Arrange
		let mut dto = relayer_transaction_dto();
		dto.transaction_hash = Some(String::new());
		dto.from = Some(String::new());
		dto.to = Some(String::new());
		dto.proxy_address = Some(String::new());
		dto.data = Some(String::new());
		dto.nonce = Some(String::new());
		dto.value = Some(String::new());
		dto.transaction_type = Some(String::new());
		dto.metadata = Some(String::new());
		dto.signature = Some(String::new());
		dto.owner = Some(String::new());
		dto.created_at = Some(String::new());
		dto.updated_at = Some(String::new());

		// Act
		let transaction = RelayerTransaction::try_from(dto).unwrap();

		// Assert
		assert_eq!(transaction.transaction_hash(), None);
		assert_eq!(transaction.from(), None);
		assert_eq!(transaction.to(), None);
		assert_eq!(transaction.proxy_address(), None);
		assert_eq!(transaction.data(), None);
		assert_eq!(transaction.nonce(), None);
		assert_eq!(transaction.value(), None);
		assert_eq!(transaction.transaction_kind(), None);
		assert_eq!(transaction.metadata(), None);
		assert_eq!(transaction.signature(), None);
		assert_eq!(transaction.owner(), None);
		assert_eq!(transaction.created_at(), None);
		assert_eq!(transaction.updated_at(), None);
	}

	#[test]
	fn submitted_transaction_rejects_invalid_uuid() {
		// Arrange
		let mut dto = submit_response_dto();
		dto.transaction_id = "not-a-uuid".to_owned();

		// Act
		let result = SubmittedTransaction::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}

	#[test]
	fn relayer_transaction_rejects_invalid_address() {
		// Arrange
		let mut dto = relayer_transaction_dto();
		dto.from = Some("not-an-address".to_owned());

		// Act
		let result = RelayerTransaction::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}

	#[test]
	fn relayer_transaction_rejects_invalid_hash() {
		// Arrange
		let mut dto = relayer_transaction_dto();
		dto.transaction_hash = Some("0x1234".to_owned());

		// Act
		let result = RelayerTransaction::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}

	#[test]
	fn relayer_transaction_rejects_invalid_hex_payload() {
		// Arrange
		let mut dto = relayer_transaction_dto();
		dto.data = Some("0xnothex".to_owned());

		// Act
		let result = RelayerTransaction::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}

	#[test]
	fn relayer_transaction_rejects_invalid_u256() {
		// Arrange
		let mut dto = relayer_transaction_dto();
		dto.nonce = Some("not-a-number".to_owned());

		// Act
		let result = RelayerTransaction::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}

	#[test]
	fn relayer_transaction_rejects_invalid_timestamp() {
		// Arrange
		let mut dto = relayer_transaction_dto();
		dto.created_at = Some("not-a-timestamp".to_owned());

		// Act
		let result = RelayerTransaction::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}

	#[test]
	fn relayer_transaction_rejects_unknown_state() {
		// Arrange
		let mut dto = relayer_transaction_dto();
		dto.state = "STATE_UNKNOWN".to_owned();

		// Act
		let result = RelayerTransaction::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}

	#[test]
	fn relayer_transaction_rejects_unknown_transaction_kind() {
		// Arrange
		let mut dto = relayer_transaction_dto();
		dto.transaction_type = Some("UNKNOWN".to_owned());

		// Act
		let result = RelayerTransaction::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}

	#[test]
	fn relayer_api_key_rejects_empty_required_field() {
		// Arrange
		let mut dto = relayer_api_key_dto();
		dto.created_at = String::new();

		// Act
		let result = RelayerApiKey::try_from(dto);

		// Assert
		assert!(matches!(result, Err(PolyrelError::Deserialize(_))));
	}
}
