//! Crate error types.

use std::borrow::Cow;

/// Errors produced by this crate.
#[derive(Debug, thiserror::Error)]
pub enum PolyrelError {
	/// HTTP transport failure.
	#[error("http error: {0}")]
	Http(Cow<'static, str>),

	/// Non-success status from the API.
	#[error("api error {status}: {body}")]
	Api {
		/// HTTP status code.
		status: u16,

		/// Response body.
		body: Cow<'static, str>,
	},

	/// Rate limited by the API (HTTP 429).
	#[error("rate limit error")]
	RateLimited,

	/// JSON deserialization failure.
	#[error("deserialize error: {0}")]
	Deserialize(Cow<'static, str>),

	/// Signing failure.
	#[error("signing error: {0}")]
	Signing(Cow<'static, str>),

	/// Invalid signature format.
	#[error("invalid signature error: {0}")]
	InvalidSignature(Cow<'static, str>),

	/// A required numeric field could not be parsed.
	#[error("invalid numeric field {field}: {value}")]
	InvalidNumericField {
		/// Field name.
		field: &'static str,

		/// Raw value that failed to parse.
		value: Cow<'static, str>,
	},

	/// Invalid header value for authentication.
	#[error("invalid auth header {header}: {detail}")]
	InvalidAuthHeader {
		/// Header name.
		header: &'static str,

		/// Detail.
		detail: Cow<'static, str>,
	},

	/// Batch must contain at least one transaction.
	#[error("empty transaction batch error")]
	EmptyBatch,

	/// Safe wallet is already deployed.
	#[error("safe already deployed error")]
	SafeAlreadyDeployed,
}

impl From<reqwest::Error> for PolyrelError {
	fn from(err: reqwest::Error) -> Self {
		Self::Http(Cow::Owned(err.to_string()))
	}
}

impl From<serde_json::Error> for PolyrelError {
	fn from(err: serde_json::Error) -> Self {
		Self::Deserialize(Cow::Owned(err.to_string()))
	}
}
