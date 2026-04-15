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

impl PolyrelError {
	/// Construct a [`PolyrelError::Http`] from anything convertible into `Cow<'static, str>`.
	pub fn http<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Http(Cow::from(msg))
	}

	/// Construct a [`PolyrelError::Deserialize`] from anything convertible into `Cow<'static, str>`.
	pub fn deserialize<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Deserialize(Cow::from(msg))
	}

	/// Construct a [`PolyrelError::Signing`] from anything convertible into `Cow<'static, str>`.
	pub fn signing<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Signing(Cow::from(msg))
	}

	/// Construct a [`PolyrelError::InvalidSignature`] from anything convertible into `Cow<'static, str>`.
	pub fn invalid_signature<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::InvalidSignature(Cow::from(msg))
	}
}

impl From<reqwest::Error> for PolyrelError {
	fn from(err: reqwest::Error) -> Self {
		Self::http(err.to_string())
	}
}

impl From<serde_json::Error> for PolyrelError {
	fn from(err: serde_json::Error) -> Self {
		Self::deserialize(err.to_string())
	}
}
