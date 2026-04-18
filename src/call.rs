use alloy_primitives::{Address, Bytes, U256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
	to: Address,
	data: Bytes,
	value: U256,
}

#[bon::bon]
impl Call {
	#[builder]
	pub fn new(to: Address, data: Bytes, value: Option<U256>) -> Self {
		Self { to, data, value: value.unwrap_or(U256::ZERO) }
	}

	pub fn to(&self) -> Address {
		self.to
	}

	pub fn data(&self) -> &Bytes {
		&self.data
	}

	pub fn value(&self) -> U256 {
		self.value
	}
}
