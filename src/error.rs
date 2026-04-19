use alloc::borrow::Cow;

/// Error type for payload construction and relayer client operations.
#[derive(Debug, thiserror::Error)]
pub enum PolyrelError {
	/// Validation failed before a payload or request could be constructed.
	#[error("validation error: {0}")]
	Validation(Cow<'static, str>),

	/// A [`crate::NonEmptyCalls`] collection was constructed from an empty vector.
	#[error("empty calls error")]
	EmptyCalls,

	/// A signature was malformed or incompatible with the expected format.
	#[error("invalid signature: {0}")]
	InvalidSignature(Cow<'static, str>),

	/// Serialization into a JSON or wire-format representation failed.
	#[error("serialization error: {0}")]
	Serialize(Cow<'static, str>),

	/// Deserialization or DTO-to-domain conversion failed.
	#[error("deserialization error: {0}")]
	Deserialize(Cow<'static, str>),

	/// The underlying HTTP client returned a transport-level error.
	#[error("http error: {0}")]
	Http(Cow<'static, str>),

	/// The relayer returned a non-success HTTP response.
	#[error("api error {status}: {body}")]
	Api {
		/// HTTP status code returned by the relayer.
		status: u16,
		/// Response body returned by the relayer.
		body: Cow<'static, str>,
	},
}

impl PolyrelError {
	/// Creates a [`Self::Validation`] error.
	pub fn validation<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Validation(Cow::from(msg))
	}

	/// Creates an [`Self::InvalidSignature`] error.
	pub fn invalid_signature<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::InvalidSignature(Cow::from(msg))
	}

	/// Creates a [`Self::Serialize`] error.
	pub fn serialize<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Serialize(Cow::from(msg))
	}

	/// Creates a [`Self::Deserialize`] error.
	pub fn deserialize<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Deserialize(Cow::from(msg))
	}

	/// Creates an [`Self::Http`] error.
	pub fn http<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Http(Cow::from(msg))
	}
}
