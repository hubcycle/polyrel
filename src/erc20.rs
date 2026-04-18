use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{SolCall, sol};

use crate::Call;

sol! {
	interface IERC20 {
		function approve(address spender, uint256 amount) external returns (bool);
		function transfer(address recipient, uint256 amount) external returns (bool);
	}
}

pub fn approve(token: Address, spender: Address, amount: U256) -> Call {
	let data = Bytes::from(IERC20::approveCall { spender, amount }.abi_encode());

	Call::builder().to(token).data(data).build()
}

pub fn transfer(token: Address, recipient: Address, amount: U256) -> Call {
	let data = Bytes::from(IERC20::transferCall { recipient, amount }.abi_encode());

	Call::builder().to(token).data(data).build()
}

#[cfg(test)]
mod tests {
	use alloy_primitives::{Bytes, U256, address};

	use super::*;

	#[test]
	fn approve_encodes_expected_calldata() {
		// Arrange
		let token = address!("1111111111111111111111111111111111111111");
		let spender = address!("2222222222222222222222222222222222222222");
		let amount = U256::from(42_u64);
		let expected = bytes(
			"095ea7b30000000000000000000000002222222222222222222222222222222222222222000000000000000000000000000000000000000000000000000000000000002a",
		);

		// Act
		let call = approve(token, spender, amount);

		// Assert
		assert_eq!(call.to(), token);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	#[test]
	fn transfer_encodes_expected_calldata() {
		// Arrange
		let token = address!("3333333333333333333333333333333333333333");
		let recipient = address!("4444444444444444444444444444444444444444");
		let amount = U256::from(7_u64);
		let expected = bytes(
			"a9059cbb00000000000000000000000044444444444444444444444444444444444444440000000000000000000000000000000000000000000000000000000000000007",
		);

		// Act
		let call = transfer(token, recipient, amount);

		// Assert
		assert_eq!(call.to(), token);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	fn bytes(hex: &str) -> Bytes {
		let decoded = alloy_primitives::hex::decode(hex).expect("valid calldata fixture");

		Bytes::from(decoded)
	}
}
