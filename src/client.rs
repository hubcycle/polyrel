//! HTTP client for the Polymarket relayer API.

use core::time::Duration;

use std::borrow::Cow;

use alloy_primitives::{Address, B256, U256};
use alloy_signer::Signer;
use reqwest::{StatusCode, header::CONTENT_TYPE};
use serde::Deserialize;

use crate::{
	auth::Auth,
	error::PolyrelError,
	sign,
	types::{
		Config, DeployedResponse, KnownTransactionState, Nonce, RelayerInfo, RelayerTransaction,
		SubmitRequest, SubmitResponse, TransactionId, WalletType,
	},
};

const PATH_SUBMIT: &str = "submit";
const PATH_TRANSACTION: &str = "transaction";
const PATH_TRANSACTIONS: &str = "transactions";
const PATH_NONCE: &str = "nonce";
const PATH_RELAY_PAYLOAD: &str = "relay-payload";
const PATH_DEPLOYED: &str = "deployed";
const QUERY_ID: &str = "id";
const QUERY_ADDRESS: &str = "address";
const QUERY_TYPE: &str = "type";
const APPLICATION_JSON: &str = "application/json";
const NONCE_TYPE_SAFE: &str = "SAFE";
const NONCE_TYPE_PROXY: &str = "PROXY";
const DEFAULT_MAX_POLLS: u32 = 10;
const DEFAULT_POLL_INTERVAL_MS: u64 = 2000;
const MIN_POLL_INTERVAL_MS: u64 = 1000;

/// Marker: client has no credentials attached.
pub struct Unauthenticated;

/// Marker: client has credentials attached.
pub struct Authenticated(Auth);

/// HTTP client for the Polymarket relayer.
///
/// The type parameter enforces that only authenticated clients can submit
/// transactions. Call [`authenticate`](RelayerClient::authenticate) to
/// transition from [`Unauthenticated`] to [`Authenticated`].
pub struct RelayerClient<State = Unauthenticated> {
	http: reqwest::Client,
	config: Config,
	state: State,
}

#[bon::bon]
impl RelayerClient<Unauthenticated> {
	/// Create a new unauthenticated client.
	#[builder]
	pub fn new(
		base_url: Option<Cow<'static, str>>,
		chain_id: Option<crate::types::ChainId>,
		ctf_exchange: Option<Address>,
		neg_risk_ctf_exchange: Option<Address>,
		neg_risk_adapter: Option<Address>,
		conditional_tokens: Option<Address>,
		usdc_e: Option<Address>,
		proxy_wallet_factory: Option<Address>,
		relay_hub: Option<Address>,
		safe_factory: Option<Address>,
		safe_multisend: Option<Address>,
		safe_init_code_hash: Option<alloy_primitives::B256>,
		proxy_init_code_hash: Option<alloy_primitives::B256>,
		http: Option<reqwest::Client>,
	) -> Result<Self, PolyrelError> {
		let config = Config::builder()
			.maybe_base_url(base_url)
			.maybe_chain_id(chain_id)
			.maybe_ctf_exchange(ctf_exchange)
			.maybe_neg_risk_ctf_exchange(neg_risk_ctf_exchange)
			.maybe_neg_risk_adapter(neg_risk_adapter)
			.maybe_conditional_tokens(conditional_tokens)
			.maybe_usdc_e(usdc_e)
			.maybe_proxy_wallet_factory(proxy_wallet_factory)
			.maybe_relay_hub(relay_hub)
			.maybe_safe_factory(safe_factory)
			.maybe_safe_multisend(safe_multisend)
			.maybe_safe_init_code_hash(safe_init_code_hash)
			.maybe_proxy_init_code_hash(proxy_init_code_hash)
			.build()?;

		Ok(Self { http: http.unwrap_or_default(), config, state: Unauthenticated })
	}

	/// Attach authentication credentials, returning an authenticated client.
	pub fn authenticate(self, auth: Auth) -> RelayerClient<Authenticated> {
		RelayerClient { http: self.http, config: self.config, state: Authenticated(auth) }
	}
}

impl<S> RelayerClient<S> {
	/// Query transaction status by ID. Returns a list of transaction records.
	pub async fn transaction(
		&self,
		id: &TransactionId,
	) -> Result<Vec<RelayerTransaction>, PolyrelError> {
		let url = self.endpoint(PATH_TRANSACTION);
		let resp = self.http.get(url).query(&[(QUERY_ID, id.as_str())]).send().await?;
		handle_response(resp).await
	}

	/// Get the current nonce for a signer's Safe wallet.
	pub async fn safe_nonce(&self, signer_address: Address) -> Result<Nonce, PolyrelError> {
		self.nonce_for(signer_address, NONCE_TYPE_SAFE).await
	}

	/// Get the current nonce for a signer's Proxy wallet.
	pub async fn proxy_nonce(&self, signer_address: Address) -> Result<Nonce, PolyrelError> {
		self.nonce_for(signer_address, NONCE_TYPE_PROXY).await
	}

	/// Get the relayer address and its nonce.
	pub async fn relay_payload(
		&self,
		signer_address: Address,
		wallet_type: WalletType,
	) -> Result<RelayerInfo, PolyrelError> {
		let url = self.endpoint(PATH_RELAY_PAYLOAD);
		let type_str = match wallet_type {
			WalletType::Safe | WalletType::SafeCreate => NONCE_TYPE_SAFE,
			WalletType::Proxy => NONCE_TYPE_PROXY,
		};
		let resp = self
			.http
			.get(url)
			.query(&[
				(QUERY_ADDRESS, signer_address.to_string()),
				(QUERY_TYPE, type_str.to_owned()),
			])
			.send()
			.await?;
		handle_response(resp).await
	}

	/// Check whether a Safe wallet has been deployed for the given address.
	pub async fn deployed(&self, address: Address) -> Result<bool, PolyrelError> {
		let url = self.endpoint(PATH_DEPLOYED);
		let resp = self.http.get(url).query(&[(QUERY_ADDRESS, address.to_string())]).send().await?;
		let payload: DeployedResponse = handle_response(resp).await?;
		Ok(payload.deployed)
	}

	/// Poll transaction status until it reaches one of the target states.
	///
	/// Returns the transaction if a target state is reached, or `None` if
	/// the fail state is hit or polling times out.
	pub async fn poll_until_state(
		&self,
		transaction_id: &TransactionId,
		target_states: &[KnownTransactionState],
		fail_state: Option<KnownTransactionState>,
		max_polls: Option<u32>,
		poll_interval: Option<Duration>,
	) -> Result<Option<RelayerTransaction>, PolyrelError> {
		let max = max_polls.unwrap_or(DEFAULT_MAX_POLLS);
		let interval = poll_interval
			.unwrap_or(Duration::from_millis(DEFAULT_POLL_INTERVAL_MS))
			.max(Duration::from_millis(MIN_POLL_INTERVAL_MS));

		for i in 0..max {
			let txns = self.transaction(transaction_id).await?;
			if let Some(txn) = txns.into_iter().next() {
				if target_states.iter().any(|s| txn.state.is(*s)) {
					return Ok(Some(txn));
				}
				if fail_state.is_some_and(|fs| txn.state.is(fs)) {
					return Ok(None);
				}
			}
			if i + 1 < max {
				tokio::time::sleep(interval).await;
			}
		}

		Ok(None)
	}

	/// Access the underlying configuration.
	pub fn config(&self) -> &Config {
		&self.config
	}

	async fn nonce_for(
		&self,
		signer_address: Address,
		wallet_type: &str,
	) -> Result<Nonce, PolyrelError> {
		#[derive(Deserialize)]
		struct NonceResponse {
			nonce: Nonce,
		}

		let url = self.endpoint(PATH_NONCE);
		let resp = self
			.http
			.get(url)
			.query(&[
				(QUERY_ADDRESS, signer_address.to_string()),
				(QUERY_TYPE, wallet_type.to_owned()),
			])
			.send()
			.await?;
		let payload: NonceResponse = handle_response(resp).await?;
		Ok(payload.nonce)
	}

	fn endpoint(&self, path: &str) -> url::Url {
		let mut url = self.config.base_url().clone();
		url.path_segments_mut().expect("URL supports path segments").push(path);
		url
	}
}

impl RelayerClient<Authenticated> {
	/// Submit a transaction to the relayer.
	pub async fn submit(&self, request: &SubmitRequest) -> Result<SubmitResponse, PolyrelError> {
		let body = serde_json::to_string(request).map_err(PolyrelError::from)?;
		let url = self.endpoint(PATH_SUBMIT);
		let path = url.path().to_owned();

		let auth_headers = self.state.0.headers(reqwest::Method::POST.as_str(), &path, &body)?;

		let resp = self
			.http
			.post(url)
			.headers(auth_headers)
			.header(CONTENT_TYPE, APPLICATION_JSON)
			.body(body)
			.send()
			.await?;
		handle_response(resp).await
	}

	/// List all transactions (authenticated).
	pub async fn transactions(&self) -> Result<Vec<RelayerTransaction>, PolyrelError> {
		let url = self.endpoint(PATH_TRANSACTIONS);
		let path = url.path().to_owned();
		let auth_headers = self.state.0.headers(reqwest::Method::GET.as_str(), &path, "")?;

		let resp = self.http.get(url).headers(auth_headers).send().await?;
		handle_response(resp).await
	}

	/// Sign a Safe transaction and submit it.
	///
	/// The Safe address is derived from the signer + factory.
	pub async fn sign_and_submit_safe<S: Signer + Sync>(
		&self,
		signer: &S,
		tx: sign::SafeTransaction,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let request = sign::sign_safe_transaction(signer, &self.config, tx, nonce.raw()).await?;
		self.submit(&request).await
	}

	/// Approve the CTF Exchange to spend USDC.e.
	pub async fn approve_usdc_for_exchange<S: Signer + Sync>(
		&self,
		signer: &S,
		amount: U256,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let (to, data) = sign::usdc_approve_exchange(&self.config, amount);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Approve the Neg-Risk CTF Exchange to spend USDC.e.
	pub async fn approve_usdc_for_neg_risk_exchange<S: Signer + Sync>(
		&self,
		signer: &S,
		amount: U256,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let (to, data) = sign::usdc_approve_neg_risk_exchange(&self.config, amount);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Transfer USDC.e to a recipient.
	pub async fn transfer_usdc<S: Signer + Sync>(
		&self,
		signer: &S,
		recipient: Address,
		amount: U256,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let (to, data) = sign::usdc_transfer(&self.config, recipient, amount);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Approve the CTF Exchange as operator for Conditional Tokens.
	pub async fn approve_ctf_for_exchange<S: Signer + Sync>(
		&self,
		signer: &S,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let (to, data) = sign::ctf_approve_exchange(&self.config);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Approve the Neg-Risk CTF Exchange as operator for Conditional Tokens.
	pub async fn approve_ctf_for_neg_risk_exchange<S: Signer + Sync>(
		&self,
		signer: &S,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let (to, data) = sign::ctf_approve_neg_risk_exchange(&self.config);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Transfer a conditional token position (ERC-1155) from the Safe wallet.
	pub async fn transfer_ctf_position<S: Signer + Sync>(
		&self,
		signer: &S,
		recipient: Address,
		token_id: U256,
		amount: U256,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let safe = sign::derive_safe_address(
			signer.address(),
			self.config.safe_factory(),
			self.config.safe_init_code_hash(),
		);
		let (to, data) = sign::ctf_transfer(&self.config, safe, recipient, token_id, amount);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Split collateral into conditional outcome tokens.
	pub async fn split_position<S: Signer + Sync>(
		&self,
		signer: &S,
		condition_id: B256,
		partition: Vec<U256>,
		amount: U256,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let (to, data) = sign::ctf_split_position(&self.config, condition_id, partition, amount);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Merge conditional outcome tokens back into collateral.
	pub async fn merge_positions<S: Signer + Sync>(
		&self,
		signer: &S,
		condition_id: B256,
		partition: Vec<U256>,
		amount: U256,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let (to, data) = sign::ctf_merge_positions(&self.config, condition_id, partition, amount);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Redeem resolved outcome tokens for collateral.
	pub async fn redeem_positions<S: Signer + Sync>(
		&self,
		signer: &S,
		condition_id: B256,
		index_sets: Vec<U256>,
		nonce: Nonce,
	) -> Result<SubmitResponse, PolyrelError> {
		let (to, data) = sign::ctf_redeem_positions(&self.config, condition_id, index_sets);
		let tx = call_tx(to, data);
		self.sign_and_submit_safe(signer, tx, nonce).await
	}

	/// Sign and submit a Safe-create (deployment) request.
	///
	/// Checks whether the Safe is already deployed first. Returns
	/// [`PolyrelError::SafeAlreadyDeployed`] if so. Signs the
	/// `CreateProxy` EIP-712 typed data and derives the Safe address
	/// from the signer + factory.
	pub async fn deploy_safe<S: Signer + Sync>(
		&self,
		signer: &S,
	) -> Result<SubmitResponse, PolyrelError> {
		let safe_address = sign::derive_safe_address(
			signer.address(),
			self.config.safe_factory(),
			self.config.safe_init_code_hash(),
		);
		if self.deployed(safe_address).await? {
			return Err(PolyrelError::SafeAlreadyDeployed);
		}
		let request = sign::sign_safe_create_request(signer, &self.config).await?;
		self.submit(&request).await
	}
}

impl From<RelayerClient<Authenticated>> for RelayerClient<Unauthenticated> {
	fn from(client: RelayerClient<Authenticated>) -> Self {
		Self { http: client.http, config: client.config, state: Unauthenticated }
	}
}

fn call_tx(to: Address, data: alloy_primitives::Bytes) -> sign::SafeTransaction {
	sign::SafeTransaction::builder().to(to).data(data.to_vec()).build()
}

async fn handle_response<T: serde::de::DeserializeOwned>(
	resp: reqwest::Response,
) -> Result<T, PolyrelError> {
	let status = resp.status();
	if status == StatusCode::TOO_MANY_REQUESTS {
		return Err(PolyrelError::RateLimited);
	}
	if !status.is_success() {
		let body = resp.text().await.unwrap_or_default();
		return Err(PolyrelError::Api { status: status.as_u16(), body: Cow::Owned(body) });
	}
	let body = resp.text().await?;
	serde_json::from_str(&body).map_err(PolyrelError::from)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn make_client(base_url: &str) -> RelayerClient<Unauthenticated> {
		RelayerClient::builder().base_url(Cow::Owned(base_url.to_owned())).build().unwrap()
	}

	#[test]
	fn endpoint_with_root_url_produces_clean_path() {
		// Arrange
		let client = make_client("https://relayer.example.com");

		// Act
		let url = client.endpoint(PATH_SUBMIT);

		// Assert
		assert_eq!(url.path(), "/submit");
	}

	#[test]
	fn endpoint_with_path_prefix_produces_clean_path() {
		// Arrange
		let client = make_client("https://example.com/api/v2");

		// Act
		let url = client.endpoint(PATH_TRANSACTION);

		// Assert
		assert_eq!(url.path(), "/api/v2/transaction");
	}

	#[test]
	fn endpoint_with_trailing_slash_prefix_produces_clean_path() {
		// Arrange
		let client = make_client("https://example.com/api/");

		// Act
		let url = client.endpoint(PATH_SUBMIT);

		// Assert
		assert_eq!(url.path(), "/api/submit");
		assert!(!url.path().contains("//"));
	}

	#[test]
	fn endpoint_with_multiple_trailing_slashes_produces_clean_path() {
		// Arrange
		let client = make_client("https://example.com/api///");

		// Act
		let url = client.endpoint(PATH_DEPLOYED);

		// Assert
		assert_eq!(url.path(), "/api/deployed");
	}

	#[test]
	fn custom_init_code_hashes_propagate_through_builder() {
		// Arrange
		let custom_safe = alloy_primitives::B256::repeat_byte(0xaa);
		let custom_proxy = alloy_primitives::B256::repeat_byte(0xbb);

		// Act
		let client = RelayerClient::builder()
			.safe_init_code_hash(custom_safe)
			.proxy_init_code_hash(custom_proxy)
			.build()
			.unwrap();

		// Assert
		assert_eq!(client.config().safe_init_code_hash(), custom_safe);
		assert_eq!(client.config().proxy_init_code_hash(), custom_proxy);
	}
}
