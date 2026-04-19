use alloc::string::String;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct NonceResponse {
	pub(super) nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RelayPayloadResponse {
	pub(super) address: String,
	pub(super) nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct SubmitResponse {
	#[serde(rename = "transactionID")]
	pub(super) transaction_id: String,

	pub(super) state: String,

	#[serde(default)]
	pub(super) hash: Option<String>,

	#[serde(rename = "transactionHash", default)]
	pub(super) transaction_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RelayerTransaction {
	#[serde(rename = "transactionID")]
	pub(super) transaction_id: String,

	#[serde(rename = "transactionHash", default)]
	pub(super) transaction_hash: Option<String>,

	#[serde(default)]
	pub(super) from: Option<String>,

	#[serde(default)]
	pub(super) to: Option<String>,

	#[serde(rename = "proxyAddress", default)]
	pub(super) proxy_address: Option<String>,

	#[serde(default)]
	pub(super) data: Option<String>,

	#[serde(default)]
	pub(super) nonce: Option<String>,

	#[serde(default)]
	pub(super) value: Option<String>,

	pub(super) state: String,

	#[serde(rename = "type", default)]
	pub(super) transaction_type: Option<String>,

	#[serde(default)]
	pub(super) metadata: Option<String>,

	#[serde(default)]
	pub(super) signature: Option<String>,

	#[serde(default)]
	pub(super) owner: Option<String>,

	#[serde(rename = "createdAt", default)]
	pub(super) created_at: Option<String>,

	#[serde(rename = "updatedAt", default)]
	pub(super) updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RelayerApiKeyRecord {
	#[serde(rename = "apiKey")]
	pub(super) api_key: String,

	pub(super) address: String,

	#[serde(rename = "createdAt")]
	pub(super) created_at: String,

	#[serde(rename = "updatedAt")]
	pub(super) updated_at: String,
}
