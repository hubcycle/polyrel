#![cfg(feature = "client")]

use alloc::{borrow::Cow, string::String, vec::Vec};

use alloy_primitives::Address;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

use crate::{PolyrelError, safe::SubmitRequest};

const QUERY_ADDRESS: &str = "address";
const QUERY_ID: &str = "id";
const QUERY_TYPE: &str = "type";

pub struct Unauthenticated;

pub struct Authenticated {
	auth: RelayerApiKeyAuth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletQueryKind {
	Safe,
	Proxy,
}

#[derive(Clone)]
pub struct RelayerApiKeyAuth {
	key: SecretString,
	address: Address,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayerBaseUrl {
	url: Url,
}

pub struct RelayerClient<State = Unauthenticated> {
	base_url: RelayerBaseUrl,
	http: reqwest::Client,
	state: State,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NonceResponse {
	pub nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RelayPayloadResponse {
	pub address: String,
	pub nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SubmitResponse {
	#[serde(rename = "transactionID")]
	pub transaction_id: String,

	pub state: String,

	#[serde(default)]
	pub hash: Option<String>,

	#[serde(rename = "transactionHash", default)]
	pub transaction_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RelayerTransaction {
	#[serde(rename = "transactionID")]
	pub transaction_id: String,

	#[serde(rename = "transactionHash", default)]
	pub transaction_hash: Option<String>,

	#[serde(default)]
	pub from: Option<String>,

	#[serde(default)]
	pub to: Option<String>,

	#[serde(rename = "proxyAddress", default)]
	pub proxy_address: Option<String>,

	#[serde(default)]
	pub data: Option<String>,

	#[serde(default)]
	pub nonce: Option<String>,

	#[serde(default)]
	pub value: Option<String>,

	pub state: String,

	#[serde(rename = "type", default)]
	pub transaction_type: Option<String>,

	#[serde(default)]
	pub metadata: Option<String>,

	#[serde(default)]
	pub signature: Option<String>,

	#[serde(default)]
	pub owner: Option<String>,

	#[serde(rename = "createdAt", default)]
	pub created_at: Option<String>,

	#[serde(rename = "updatedAt", default)]
	pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RelayerApiKeyRecord {
	#[serde(rename = "apiKey")]
	pub api_key: String,

	pub address: String,

	#[serde(rename = "createdAt")]
	pub created_at: String,

	#[serde(rename = "updatedAt")]
	pub updated_at: String,
}

impl WalletQueryKind {
	const SAFE_KIND: &str = "SAFE";
	const PROXY_KIND: &str = "PROXY";

	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Safe => Self::SAFE_KIND,
			Self::Proxy => Self::PROXY_KIND,
		}
	}
}

impl RelayerApiKeyAuth {
	pub const HEADER_API_KEY: &str = "RELAYER_API_KEY";
	pub const HEADER_API_KEY_ADDRESS: &str = "RELAYER_API_KEY_ADDRESS";
	const HEADER_API_KEY_LOWER: &str = "relayer_api_key";
	const HEADER_API_KEY_ADDRESS_LOWER: &str = "relayer_api_key_address";

	pub fn new(key: SecretString, address: Address) -> Self {
		Self { key, address }
	}

	pub fn key(&self) -> &SecretString {
		&self.key
	}

	pub fn address(&self) -> Address {
		self.address
	}
}

impl RelayerBaseUrl {
	const SCHEME_HTTP: &str = "http";
	const SCHEME_HTTPS: &str = "https";

	pub fn new(mut url: Url) -> Result<Self, PolyrelError> {
		match url.scheme() {
			Self::SCHEME_HTTP | Self::SCHEME_HTTPS => {},
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

	pub fn parse<U>(url: U) -> Result<Self, PolyrelError>
	where
		U: AsRef<str>,
	{
		let url = Url::parse(url.as_ref()).map_err(|e| PolyrelError::validation(e.to_string()))?;

		Self::new(url)
	}

	pub fn as_url(&self) -> &Url {
		&self.url
	}
}

impl RelayerClient<Unauthenticated> {
	pub fn new(base_url: RelayerBaseUrl) -> Self {
		Self { base_url, http: reqwest::Client::new(), state: Unauthenticated }
	}

	pub fn with_http(base_url: RelayerBaseUrl, http: reqwest::Client) -> Self {
		Self { base_url, http, state: Unauthenticated }
	}

	pub fn authenticate(self, auth: RelayerApiKeyAuth) -> RelayerClient<Authenticated> {
		RelayerClient { base_url: self.base_url, http: self.http, state: Authenticated { auth } }
	}
}

impl<S> RelayerClient<S> {
	const CONTENT_TYPE_JSON: &str = "application/json";
	const PATH_DEPLOYED: &'static [&'static str] = &["deployed"];
	const PATH_NONCE: &'static [&'static str] = &["nonce"];
	const PATH_RELAY_PAYLOAD: &'static [&'static str] = &["relay-payload"];
	const PATH_RELAYER_API_KEYS: &'static [&'static str] = &["relayer", "api", "keys"];
	const PATH_SUBMIT: &'static [&'static str] = &["submit"];
	const PATH_TRANSACTION: &'static [&'static str] = &["transaction"];
	const PATH_TRANSACTIONS: &'static [&'static str] = &["transactions"];

	pub fn base_url(&self) -> &RelayerBaseUrl {
		&self.base_url
	}

	pub async fn transaction_by_id(
		&self,
		transaction_id: &str,
	) -> Result<Vec<RelayerTransaction>, PolyrelError> {
		let url = self.endpoint(Self::PATH_TRANSACTION)?;
		let response = self
			.http
			.get(url)
			.query(&[(QUERY_ID, transaction_id)])
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		handle_response(response).await
	}

	pub async fn current_nonce(
		&self,
		address: Address,
		kind: WalletQueryKind,
	) -> Result<NonceResponse, PolyrelError> {
		let url = self.endpoint(Self::PATH_NONCE)?;
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

		handle_response(response).await
	}

	pub async fn relay_payload(
		&self,
		address: Address,
		kind: WalletQueryKind,
	) -> Result<RelayPayloadResponse, PolyrelError> {
		let url = self.endpoint(Self::PATH_RELAY_PAYLOAD)?;
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

		handle_response(response).await
	}

	pub async fn is_safe_deployed(&self, address: Address) -> Result<bool, PolyrelError> {
		#[derive(Deserialize)]
		struct DeployedResponse {
			deployed: bool,
		}

		let url = self.endpoint(Self::PATH_DEPLOYED)?;
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

impl RelayerClient<Authenticated> {
	pub fn auth(&self) -> &RelayerApiKeyAuth {
		&self.state.auth
	}

	pub async fn submit(&self, request: &SubmitRequest) -> Result<SubmitResponse, PolyrelError> {
		let url = self.endpoint(Self::PATH_SUBMIT)?;
		let response = self
			.http
			.post(url)
			.headers(auth_headers(self.auth())?)
			.header(CONTENT_TYPE, Self::CONTENT_TYPE_JSON)
			.json(request)
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		handle_response(response).await
	}

	pub async fn recent_transactions(&self) -> Result<Vec<RelayerTransaction>, PolyrelError> {
		let url = self.endpoint(Self::PATH_TRANSACTIONS)?;
		let response = self
			.http
			.get(url)
			.headers(auth_headers(self.auth())?)
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		handle_response(response).await
	}

	pub async fn relayer_api_keys(&self) -> Result<Vec<RelayerApiKeyRecord>, PolyrelError> {
		let url = self.endpoint(Self::PATH_RELAYER_API_KEYS)?;
		let response = self
			.http
			.get(url)
			.headers(auth_headers(self.auth())?)
			.send()
			.await
			.map_err(|e| PolyrelError::http(e.to_string()))?;

		handle_response(response).await
	}
}

impl core::fmt::Debug for RelayerApiKeyAuth {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("RelayerApiKeyAuth")
			.field("key", &"[REDACTED]")
			.field("address", &self.address)
			.finish()
	}
}

fn auth_headers(auth: &RelayerApiKeyAuth) -> Result<HeaderMap, PolyrelError> {
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
	use wiremock::{
		Mock, MockServer, ResponseTemplate,
		matchers::{body_json, header, method, path, query_param},
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
	const TRANSACTION_ID: &str = "tx-123";
	const SUBMIT_TRANSACTION_ID: &str = "tx-submit";
	const RECENT_TRANSACTION_ID: &str = "tx-1";
	const API_KEY_RECORD: &str = "01967c03-b8c8-7000-8f68-8b8eaec6fd3d";
	const API_KEY_TIMESTAMP: &str = "2026-02-24T18:20:11.237485Z";
	const MOCK_STATE_NEW: &str = "STATE_NEW";

	fn base_url(server: &MockServer) -> RelayerBaseUrl {
		RelayerBaseUrl::parse(Cow::Owned(server.uri())).unwrap()
	}

	fn auth() -> RelayerApiKeyAuth {
		RelayerApiKeyAuth::new(SecretString::from(AUTH_KEY), AUTH_ADDRESS)
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
		assert_eq!(response.nonce, NONCE_VALUE);
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
		assert_eq!(response.address, RELAY_PAYLOAD_ADDRESS);
		assert_eq!(response.nonce, RELAY_PAYLOAD_NONCE);
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
		assert_eq!(response[0].transaction_id, TRANSACTION_ID);
	}

	#[tokio::test]
	async fn submit_sends_auth_headers_and_body() {
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
		let client = RelayerClient::new(base_url(&server)).authenticate(auth());

		// Act
		let response = client.submit(&request).await.unwrap();

		// Assert
		assert_eq!(response.transaction_id, SUBMIT_TRANSACTION_ID);
	}

	#[tokio::test]
	async fn recent_transactions_sends_auth_headers() {
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
		let client = RelayerClient::new(base_url(&server)).authenticate(auth());

		// Act
		let response = client.recent_transactions().await.unwrap();

		// Assert
		assert_eq!(response.len(), 1);
		assert_eq!(response[0].transaction_id, RECENT_TRANSACTION_ID);
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
					"address": "0xabc",
					"createdAt": API_KEY_TIMESTAMP,
					"updatedAt": API_KEY_TIMESTAMP
				}
			])))
			.mount(&server)
			.await;
		let client = RelayerClient::new(base_url(&server)).authenticate(auth());

		// Act
		let response = client.relayer_api_keys().await.unwrap();

		// Assert
		assert_eq!(response.len(), 1);
		assert_eq!(response[0].api_key, API_KEY_RECORD);
	}
}
