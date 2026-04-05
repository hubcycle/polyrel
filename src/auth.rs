//! Authentication strategies for the relayer API.

use std::borrow::Cow;

use alloy_primitives::Address;
use base64::Engine as _;
use hmac::{KeyInit as _, Mac as _};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use secrecy::{ExposeSecret, SecretString};

use crate::PolyrelError;

const HEADER_BUILDER_API_KEY: HeaderName = HeaderName::from_static("poly_builder_api_key");
const HEADER_BUILDER_SIGNATURE: HeaderName = HeaderName::from_static("poly_builder_signature");
const HEADER_BUILDER_TIMESTAMP: HeaderName = HeaderName::from_static("poly_builder_timestamp");
const HEADER_BUILDER_PASSPHRASE: HeaderName = HeaderName::from_static("poly_builder_passphrase");
const RELAYER_API_KEY_STR: &str = "relayer_api_key";
const RELAYER_API_KEY_ADDRESS_STR: &str = "relayer_api_key_address";
const HEADER_RELAYER_API_KEY: HeaderName = HeaderName::from_static(RELAYER_API_KEY_STR);
const HEADER_RELAYER_API_KEY_ADDRESS: HeaderName =
	HeaderName::from_static(RELAYER_API_KEY_ADDRESS_STR);
const REDACTED: &str = "[REDACTED]";

type HmacSha256 = hmac::Hmac<sha2::Sha256>;

/// Builder API key credentials (HMAC-SHA256).
#[derive(Clone)]
pub struct BuilderCredentials {
	/// API key.
	pub api_key: SecretString,

	/// Base64-encoded HMAC secret.
	pub secret: SecretString,

	/// Passphrase.
	pub passphrase: SecretString,
}

/// Simple relayer API key.
#[derive(Clone)]
pub struct RelayerApiKey {
	/// API key string.
	pub key: SecretString,

	/// Address that owns the key.
	pub address: Address,
}

/// Authentication strategy for the relayer.
#[derive(Debug, Clone)]
pub enum Auth {
	/// HMAC-SHA256 builder credentials.
	Builder(BuilderCredentials),

	/// Simple API key.
	Relayer(RelayerApiKey),
}

impl Auth {
	/// Produce a [`HeaderMap`] for an authenticated request.
	pub(crate) fn headers(
		&self,
		method: &str,
		path: &str,
		body: &str,
	) -> Result<HeaderMap, PolyrelError> {
		match self {
			Self::Builder(creds) => builder_headers(creds, method, path, body),
			Self::Relayer(key) => relayer_headers(key),
		}
	}
}

impl core::fmt::Debug for BuilderCredentials {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("BuilderCredentials")
			.field("api_key", &REDACTED)
			.field("secret", &REDACTED)
			.field("passphrase", &REDACTED)
			.finish()
	}
}

impl core::fmt::Debug for RelayerApiKey {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("RelayerApiKey")
			.field("key", &REDACTED)
			.field("address", &self.address)
			.finish()
	}
}

fn builder_headers(
	creds: &BuilderCredentials,
	method: &str,
	path: &str,
	body: &str,
) -> Result<HeaderMap, PolyrelError> {
	let timestamp = timestamp_secs();
	let message = format!("{timestamp}{method}{path}{body}");

	let secret_bytes = decode_builder_secret(creds.secret.expose_secret())
		.map_err(|e| PolyrelError::Signing(Cow::Owned(format!("base64 decode: {e}"))))?;

	let mut mac = HmacSha256::new_from_slice(&secret_bytes)
		.map_err(|e| PolyrelError::Signing(Cow::Owned(format!("hmac init: {e}"))))?;
	mac.update(message.as_bytes());
	let signature = url_safe_base64(mac.finalize().into_bytes().as_slice());

	let mut headers = HeaderMap::new();
	headers.insert(
		HEADER_BUILDER_API_KEY,
		HeaderValue::from_str(creds.api_key.expose_secret())
			.map_err(|e| PolyrelError::Http(Cow::Owned(e.to_string())))?,
	);
	headers.insert(
		HEADER_BUILDER_PASSPHRASE,
		HeaderValue::from_str(creds.passphrase.expose_secret())
			.map_err(|e| PolyrelError::Http(Cow::Owned(e.to_string())))?,
	);
	headers.insert(
		HEADER_BUILDER_SIGNATURE,
		HeaderValue::from_str(&signature)
			.map_err(|e| PolyrelError::Http(Cow::Owned(e.to_string())))?,
	);
	headers.insert(
		HEADER_BUILDER_TIMESTAMP,
		HeaderValue::from_str(&timestamp)
			.map_err(|e| PolyrelError::Http(Cow::Owned(e.to_string())))?,
	);

	Ok(headers)
}

fn relayer_headers(key: &RelayerApiKey) -> Result<HeaderMap, PolyrelError> {
	let mut headers = HeaderMap::new();
	headers.insert(
		HEADER_RELAYER_API_KEY,
		HeaderValue::from_str(key.key.expose_secret()).map_err(|e| {
			PolyrelError::InvalidAuthHeader {
				header: RELAYER_API_KEY_STR,
				detail: Cow::Owned(e.to_string()),
			}
		})?,
	);
	headers.insert(
		HEADER_RELAYER_API_KEY_ADDRESS,
		HeaderValue::from_str(&key.address.to_string()).map_err(|e| {
			PolyrelError::InvalidAuthHeader {
				header: RELAYER_API_KEY_ADDRESS_STR,
				detail: Cow::Owned(e.to_string()),
			}
		})?,
	);
	Ok(headers)
}

fn url_safe_base64(raw: &[u8]) -> String {
	base64::engine::general_purpose::STANDARD.encode(raw).replace('+', "-").replace('/', "_")
}

fn decode_builder_secret(secret: &str) -> Result<Vec<u8>, base64::DecodeError> {
	base64::engine::general_purpose::STANDARD
		.decode(secret.as_bytes())
		.or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(secret.as_bytes()))
		.or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(secret.as_bytes()))
		.or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(secret.as_bytes()))
}

fn timestamp_secs() -> String {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.expect("system clock before UNIX epoch")
		.as_secs()
		.to_string()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn builder_credentials_debug_redacts_secrets() {
		// Arrange
		let creds = BuilderCredentials {
			api_key: SecretString::from("my-key"),
			secret: SecretString::from("bXktc2VjcmV0"),
			passphrase: SecretString::from("my-pass"),
		};

		// Act
		let debug = format!("{creds:?}");

		// Assert
		assert!(!debug.contains("my-key"));
		assert!(!debug.contains("my-pass"));
		assert!(debug.contains(REDACTED));
	}

	#[test]
	fn relayer_api_key_debug_redacts_key() {
		// Arrange
		let key = RelayerApiKey { key: SecretString::from("secret-key"), address: Address::ZERO };

		// Act
		let debug = format!("{key:?}");

		// Assert
		assert!(!debug.contains("secret-key"));
		assert!(debug.contains(REDACTED));
	}

	#[test]
	fn builder_hmac_produces_url_safe_base64_signature() {
		// Arrange
		let creds = BuilderCredentials {
			api_key: SecretString::from("test-key"),
			secret: SecretString::from(
				base64::engine::general_purpose::STANDARD.encode("test-secret"),
			),
			passphrase: SecretString::from("test-pass"),
		};

		// Act
		let headers = builder_headers(&creds, "POST", "/submit", r#"{"data":"0x"}"#)
			.expect("should produce headers");

		// Assert
		assert!(headers.contains_key(HEADER_BUILDER_API_KEY));
		assert!(headers.contains_key(HEADER_BUILDER_SIGNATURE));
		assert!(headers.contains_key(HEADER_BUILDER_TIMESTAMP));
		assert!(headers.contains_key(HEADER_BUILDER_PASSPHRASE));
		let sig = headers[HEADER_BUILDER_SIGNATURE].to_str().unwrap();
		assert!(!sig.contains('+'));
		assert!(!sig.contains('/'));
	}

	#[test]
	fn url_safe_base64_replaces_plus_and_slash() {
		// Arrange
		let raw = [0xfb, 0xff, 0xfe];

		// Act
		let encoded = url_safe_base64(&raw);

		// Assert
		assert!(!encoded.contains('+'));
		assert!(!encoded.contains('/'));
		assert!(encoded.contains('-') || encoded.contains('_'));
	}

	#[test]
	fn decode_builder_secret_handles_variants() {
		// Arrange
		let standard = base64::engine::general_purpose::STANDARD.encode("test");
		let url_safe = base64::engine::general_purpose::URL_SAFE.encode("test");
		let no_pad = base64::engine::general_purpose::STANDARD_NO_PAD.encode("test");

		// Act & Assert
		assert!(decode_builder_secret(&standard).is_ok());
		assert!(decode_builder_secret(&url_safe).is_ok());
		assert!(decode_builder_secret(&no_pad).is_ok());
	}
}
