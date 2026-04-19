use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{SolCall, sol};

use crate::Call;

sol! {
	interface IERC20 {
		function approve(address spender, uint256 amount) external returns (bool);
		function transfer(address recipient, uint256 amount) external returns (bool);
	}
}

/// Builds ERC-20 `approve(spender, amount)` calldata.
pub fn approve(token: Address, spender: Address, amount: U256) -> Call {
	let data = Bytes::from(IERC20::approveCall { spender, amount }.abi_encode());

	Call::builder().to(token).data(data).build()
}

/// Builds ERC-20 `transfer(recipient, amount)` calldata.
pub fn transfer(token: Address, recipient: Address, amount: U256) -> Call {
	let data = Bytes::from(IERC20::transferCall { recipient, amount }.abi_encode());

	Call::builder().to(token).data(data).build()
}

#[cfg(test)]
mod tests {
	use alloy_primitives::{Bytes, U256, address};

	use super::*;

	const APPROVE_TOKEN: Address = address!("1111111111111111111111111111111111111111");
	const APPROVE_SPENDER: Address = address!("2222222222222222222222222222222222222222");
	const APPROVE_AMOUNT: u64 = 42;
	const APPROVE_EXPECTED: &str = "095ea7b30000000000000000000000002222222222222222222222222222222222222222000000000000000000000000000000000000000000000000000000000000002a";
	const TRANSFER_TOKEN: Address = address!("3333333333333333333333333333333333333333");
	const TRANSFER_RECIPIENT: Address = address!("4444444444444444444444444444444444444444");
	const TRANSFER_AMOUNT: u64 = 7;
	const TRANSFER_EXPECTED: &str = "a9059cbb00000000000000000000000044444444444444444444444444444444444444440000000000000000000000000000000000000000000000000000000000000007";
	const VALID_CALLDATA_FIXTURE: &str = "valid calldata fixture";

	#[test]
	fn approve_encodes_expected_calldata() {
		// Arrange
		let token = APPROVE_TOKEN;
		let spender = APPROVE_SPENDER;
		let amount = U256::from(APPROVE_AMOUNT);
		let expected = bytes(APPROVE_EXPECTED);

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
		let token = TRANSFER_TOKEN;
		let recipient = TRANSFER_RECIPIENT;
		let amount = U256::from(TRANSFER_AMOUNT);
		let expected = bytes(TRANSFER_EXPECTED);

		// Act
		let call = transfer(token, recipient, amount);

		// Assert
		assert_eq!(call.to(), token);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	fn bytes(hex: &str) -> Bytes {
		let decoded = alloy_primitives::hex::decode(hex).expect(VALID_CALLDATA_FIXTURE);

		Bytes::from(decoded)
	}
}
