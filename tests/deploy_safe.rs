//! Integration tests for Safe deployment via the relayer client.

use std::borrow::Cow;

use alloy_primitives::B256;
use alloy_signer_local::PrivateKeySigner;
use polyrel::{Auth, BuilderCredentials, PolyrelError, RelayerClient};
use secrecy::SecretString;
use wiremock::{
	Mock, MockServer, ResponseTemplate,
	matchers::{method, path, query_param},
};

fn test_auth() -> Auth {
	Auth::Builder(BuilderCredentials {
		api_key: SecretString::from("test-key"),
		secret: SecretString::from("dGVzdC1zZWNyZXQ="),
		passphrase: SecretString::from("test-pass"),
	})
}

#[tokio::test]
async fn deploy_safe_returns_already_deployed_when_safe_exists() {
	// Arrange
	let signer = PrivateKeySigner::random();
	let safe_address = polyrel::derive_safe_address(
		signer.address(),
		polyrel::SAFE_FACTORY,
		B256::from(polyrel::SAFE_INIT_CODE_HASH),
	);

	let server = MockServer::start().await;
	Mock::given(method("GET"))
		.and(path("/deployed"))
		.and(query_param("address", safe_address.to_string()))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
			"deployed": true
		})))
		.mount(&server)
		.await;

	let client = RelayerClient::builder()
		.base_url(Cow::Owned(server.uri()))
		.build()
		.unwrap()
		.authenticate(test_auth());

	// Act
	let result = client.deploy_safe(&signer).await;

	// Assert
	assert!(matches!(result, Err(PolyrelError::SafeAlreadyDeployed)));
}

#[tokio::test]
async fn deploy_safe_submits_when_safe_not_deployed() {
	// Arrange
	let signer = PrivateKeySigner::random();
	let safe_address = polyrel::derive_safe_address(
		signer.address(),
		polyrel::SAFE_FACTORY,
		B256::from(polyrel::SAFE_INIT_CODE_HASH),
	);

	let server = MockServer::start().await;
	Mock::given(method("GET"))
		.and(path("/deployed"))
		.and(query_param("address", safe_address.to_string()))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
			"deployed": false
		})))
		.mount(&server)
		.await;

	Mock::given(method("POST"))
		.and(path("/submit"))
		.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
			"transactionID": "tx-deploy-123",
			"state": "STATE_NEW"
		})))
		.mount(&server)
		.await;

	let client = RelayerClient::builder()
		.base_url(Cow::Owned(server.uri()))
		.build()
		.unwrap()
		.authenticate(test_auth());

	// Act
	let result = client.deploy_safe(&signer).await;

	// Assert
	let resp = result.unwrap();
	assert_eq!(resp.transaction_id, "tx-deploy-123");
	assert_eq!(resp.state, "STATE_NEW");
}
