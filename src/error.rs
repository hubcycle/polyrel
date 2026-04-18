use alloc::borrow::Cow;

#[derive(Debug, thiserror::Error)]
pub enum PolyrelError {
	#[error("validation error: {0}")]
	Validation(Cow<'static, str>),

	#[error("empty calls error")]
	EmptyCalls,

	#[error("invalid signature: {0}")]
	InvalidSignature(Cow<'static, str>),

	#[error("serialization error: {0}")]
	Serialize(Cow<'static, str>),

	#[error("http error: {0}")]
	Http(Cow<'static, str>),

	#[error("api error {status}: {body}")]
	Api {
		status: u16,
		body: Cow<'static, str>,
	},
}

impl PolyrelError {
	pub fn validation<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Validation(Cow::from(msg))
	}

	pub fn invalid_signature<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::InvalidSignature(Cow::from(msg))
	}

	pub fn serialize<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Serialize(Cow::from(msg))
	}

	pub fn http<E>(msg: E) -> Self
	where
		Cow<'static, str>: From<E>,
	{
		Self::Http(Cow::from(msg))
	}
}
