use core::num::NonZeroUsize;

use alloc::{vec, vec::Vec};

use alloy_primitives::{Address, Bytes, U256};

use crate::PolyrelError;

/// Generic EVM call envelope used by the builders in this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
	to: Address,
	data: Bytes,
	value: U256,
}

/// Non-empty collection of [`Call`] values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyCalls {
	calls: Vec<Call>,
}

#[bon::bon]
impl Call {
	/// Creates a new call targeting `to` with ABI-encoded `data`.
	///
	/// When `value` is omitted it defaults to zero.
	#[builder]
	pub fn new(to: Address, data: Bytes, value: Option<U256>) -> Self {
		Self { to, data, value: value.unwrap_or(U256::ZERO) }
	}

	/// Returns the destination contract address.
	pub fn to(&self) -> Address {
		self.to
	}

	/// Returns the ABI-encoded calldata.
	pub fn data(&self) -> &Bytes {
		&self.data
	}

	/// Returns the native token value to send with the call.
	pub fn value(&self) -> U256 {
		self.value
	}
}

impl NonEmptyCalls {
	/// Creates a non-empty call collection.
	pub fn new(calls: Vec<Call>) -> Result<Self, PolyrelError> {
		if calls.is_empty() {
			return Err(PolyrelError::EmptyCalls);
		}

		Ok(Self { calls })
	}

	/// Creates a one-element collection from a single call.
	pub fn from_one(call: Call) -> Self {
		Self { calls: vec![call] }
	}

	/// Returns the number of calls in the collection.
	pub fn len(&self) -> NonZeroUsize {
		NonZeroUsize::new(self.calls.len()).unwrap() // unwrap is safe as `calls` is non-empty
	}

	/// Returns the calls as a slice.
	pub fn as_slice(&self) -> &[Call] {
		&self.calls
	}

	/// Appends a call to the collection.
	pub fn push(&mut self, call: Call) {
		self.calls.push(call);
	}

	/// Consumes the wrapper and returns the underlying vector.
	pub fn into_vec(self) -> Vec<Call> {
		self.calls
	}
}

#[cfg(test)]
mod tests {
	use alloc::vec;

	use alloy_primitives::{Bytes, U256, address};

	use super::*;

	const FIXTURE_VALUE: u64 = 3;
	const FIXTURE_DATA: &[u8] = &[0xaa, 0xbb];
	const FIRST_CALL_ADDRESS: Address = address!("1111111111111111111111111111111111111111");
	const SECOND_CALL_ADDRESS: Address = address!("2222222222222222222222222222222222222222");
	const SECOND_CALL_DATA: &[u8] = &[0xcc];

	fn fixture_call() -> Call {
		Call::builder()
			.to(FIRST_CALL_ADDRESS)
			.data(Bytes::from_static(FIXTURE_DATA))
			.value(U256::from(FIXTURE_VALUE))
			.build()
	}

	#[test]
	fn new_rejects_empty_vectors() {
		// Arrange
		let calls = Vec::new();

		// Act
		let result = NonEmptyCalls::new(calls);

		// Assert
		assert!(matches!(result, Err(PolyrelError::EmptyCalls)));
	}

	#[test]
	fn new_accepts_multiple_calls_and_preserves_order() {
		// Arrange
		let first = fixture_call();
		let second = Call::builder()
			.to(SECOND_CALL_ADDRESS)
			.data(Bytes::from_static(SECOND_CALL_DATA))
			.build();

		// Act
		let calls = NonEmptyCalls::new(vec![first.clone(), second.clone()]).expect("non-empty");

		// Assert
		assert_eq!(calls.len().get(), 2);
		assert_eq!(calls.as_slice(), &[first, second]);
	}

	#[test]
	fn from_one_creates_single_item_collection() {
		// Arrange
		let call = fixture_call();

		// Act
		let calls = NonEmptyCalls::from_one(call.clone());

		// Assert
		assert_eq!(calls.len().get(), 1);
		assert_eq!(calls.as_slice(), &[call]);
	}
}
