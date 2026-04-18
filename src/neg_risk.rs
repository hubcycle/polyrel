use alloc::vec::Vec;

use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_sol_types::{SolCall, sol};

use crate::Call;

sol! {
	interface INegRiskAdapter {
		function redeemPositions(bytes32 conditionId, uint256[] amounts) external;
	}
}

pub fn redeem_positions(adapter: Address, condition_id: B256, amounts: Vec<U256>) -> Call {
	let data = Bytes::from(
		INegRiskAdapter::redeemPositionsCall { conditionId: condition_id, amounts }.abi_encode(),
	);

	Call::builder().to(adapter).data(data).build()
}

#[cfg(test)]
mod tests {
	use alloc::vec;

	use alloy_primitives::{Bytes, U256, address, b256};

	use super::*;

	#[test]
	fn redeem_positions_encodes_expected_calldata() {
		// Arrange
		let adapter = address!("1212121212121212121212121212121212121212");
		let condition_id =
			b256!("abababababababababababababababababababababababababababababababab");
		let amounts = vec![U256::from(5_u64), U256::from(9_u64)];
		let expected = bytes(
			"dbeccb23abababababababababababababababababababababababababababababababab0000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000050000000000000000000000000000000000000000000000000000000000000009",
		);

		// Act
		let call = redeem_positions(adapter, condition_id, amounts.clone());

		// Assert
		assert_eq!(call.to(), adapter);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	fn bytes(hex: &str) -> Bytes {
		let decoded = alloy_primitives::hex::decode(hex).expect("valid calldata fixture");

		Bytes::from(decoded)
	}
}
