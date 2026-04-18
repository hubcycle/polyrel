use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{SolCall, sol};

use crate::Call;

sol! {
	interface IERC1155 {
		function setApprovalForAll(address operator, bool approved) external;
		function safeTransferFrom(
			address from,
			address to,
			uint256 id,
			uint256 amount,
			bytes data
		) external;
	}
}

pub fn set_approval_for_all(token: Address, operator: Address, approved: bool) -> Call {
	let data = Bytes::from(IERC1155::setApprovalForAllCall { operator, approved }.abi_encode());

	Call::builder().to(token).data(data).build()
}

pub fn safe_transfer_from(
	token: Address,
	from: Address,
	to: Address,
	id: U256,
	amount: U256,
	data: Bytes,
) -> Call {
	let data =
		Bytes::from(IERC1155::safeTransferFromCall { from, to, id, amount, data }.abi_encode());

	Call::builder().to(token).data(data).build()
}

#[cfg(test)]
mod tests {
	use alloy_primitives::{Bytes, U256, address};

	use super::*;

	#[test]
	fn set_approval_for_all_encodes_expected_calldata() {
		// Arrange
		let token = address!("5555555555555555555555555555555555555555");
		let operator = address!("6666666666666666666666666666666666666666");
		let expected = bytes(
			"a22cb46500000000000000000000000066666666666666666666666666666666666666660000000000000000000000000000000000000000000000000000000000000001",
		);

		// Act
		let call = set_approval_for_all(token, operator, true);

		// Assert
		assert_eq!(call.to(), token);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	#[test]
	fn safe_transfer_from_encodes_expected_calldata() {
		// Arrange
		let token = address!("7777777777777777777777777777777777777777");
		let from = address!("8888888888888888888888888888888888888888");
		let to = address!("9999999999999999999999999999999999999999");
		let id = U256::from(3_u64);
		let amount = U256::from(11_u64);
		let payload = Bytes::from_static(&[0xaa, 0xbb, 0xcc]);
		let expected = bytes(
			"f242432a000000000000000000000000888888888888888888888888888888888888888800000000000000000000000099999999999999999999999999999999999999990000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000b00000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000003aabbcc0000000000000000000000000000000000000000000000000000000000",
		);

		// Act
		let call = safe_transfer_from(token, from, to, id, amount, payload.clone());

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
