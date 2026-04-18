use alloc::vec::Vec;

use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_sol_types::{SolCall, sol};

use crate::Call;

sol! {
	interface IConditionalTokens {
		function splitPosition(
			address collateralToken,
			bytes32 parentCollectionId,
			bytes32 conditionId,
			uint256[] partition,
			uint256 amount
		) external;

		function mergePositions(
			address collateralToken,
			bytes32 parentCollectionId,
			bytes32 conditionId,
			uint256[] partition,
			uint256 amount
		) external;

		function redeemPositions(
			address collateralToken,
			bytes32 parentCollectionId,
			bytes32 conditionId,
			uint256[] indexSets
		) external;
	}
}

pub fn split_position(
	ctf: Address,
	collateral_token: Address,
	parent_collection_id: B256,
	condition_id: B256,
	partition: Vec<U256>,
	amount: U256,
) -> Call {
	let data = Bytes::from(
		IConditionalTokens::splitPositionCall {
			collateralToken: collateral_token,
			parentCollectionId: parent_collection_id,
			conditionId: condition_id,
			partition,
			amount,
		}
		.abi_encode(),
	);

	Call::builder().to(ctf).data(data).build()
}

pub fn merge_positions(
	ctf: Address,
	collateral_token: Address,
	parent_collection_id: B256,
	condition_id: B256,
	partition: Vec<U256>,
	amount: U256,
) -> Call {
	let data = Bytes::from(
		IConditionalTokens::mergePositionsCall {
			collateralToken: collateral_token,
			parentCollectionId: parent_collection_id,
			conditionId: condition_id,
			partition,
			amount,
		}
		.abi_encode(),
	);

	Call::builder().to(ctf).data(data).build()
}

pub fn redeem_positions(
	ctf: Address,
	collateral_token: Address,
	parent_collection_id: B256,
	condition_id: B256,
	index_sets: Vec<U256>,
) -> Call {
	let data = Bytes::from(
		IConditionalTokens::redeemPositionsCall {
			collateralToken: collateral_token,
			parentCollectionId: parent_collection_id,
			conditionId: condition_id,
			indexSets: index_sets,
		}
		.abi_encode(),
	);

	Call::builder().to(ctf).data(data).build()
}

#[cfg(test)]
mod tests {
	use alloc::vec;

	use alloy_primitives::{B256, Bytes, U256, address, b256};

	use super::*;

	#[test]
	fn split_position_encodes_expected_calldata() {
		// Arrange
		let ctf = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
		let collateral_token = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
		let parent_collection_id = B256::ZERO;
		let condition_id =
			b256!("1111111111111111111111111111111111111111111111111111111111111111");
		let partition = vec![U256::from(1_u64), U256::from(2_u64)];
		let amount = U256::from(10_u64);
		let expected = bytes(
			"72ce4275000000000000000000000000bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb0000000000000000000000000000000000000000000000000000000000000000111111111111111111111111111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002",
		);

		// Act
		let call = split_position(
			ctf,
			collateral_token,
			parent_collection_id,
			condition_id,
			partition.clone(),
			amount,
		);

		// Assert
		assert_eq!(call.to(), ctf);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	#[test]
	fn merge_positions_encodes_expected_calldata() {
		// Arrange
		let ctf = address!("cccccccccccccccccccccccccccccccccccccccc");
		let collateral_token = address!("dddddddddddddddddddddddddddddddddddddddd");
		let parent_collection_id =
			b256!("2222222222222222222222222222222222222222222222222222222222222222");
		let condition_id =
			b256!("3333333333333333333333333333333333333333333333333333333333333333");
		let partition = vec![U256::from(4_u64), U256::from(8_u64)];
		let amount = U256::from(25_u64);
		let expected = bytes(
			"9e7212ad000000000000000000000000dddddddddddddddddddddddddddddddddddddddd2222222222222222222222222222222222222222222222222222222222222222333333333333333333333333333333333333333333333333333333333333333300000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000019000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008",
		);

		// Act
		let call = merge_positions(
			ctf,
			collateral_token,
			parent_collection_id,
			condition_id,
			partition.clone(),
			amount,
		);

		// Assert
		assert_eq!(call.to(), ctf);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	#[test]
	fn redeem_positions_encodes_expected_calldata() {
		// Arrange
		let ctf = address!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
		let collateral_token = address!("ffffffffffffffffffffffffffffffffffffffff");
		let parent_collection_id =
			b256!("4444444444444444444444444444444444444444444444444444444444444444");
		let condition_id =
			b256!("5555555555555555555555555555555555555555555555555555555555555555");
		let index_sets = vec![U256::from(1_u64), U256::from(3_u64)];
		let expected = bytes(
			"01b7037c000000000000000000000000ffffffffffffffffffffffffffffffffffffffff444444444444444444444444444444444444444444444444444444444444444455555555555555555555555555555555555555555555555555555555555555550000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000003",
		);

		// Act
		let call = redeem_positions(
			ctf,
			collateral_token,
			parent_collection_id,
			condition_id,
			index_sets.clone(),
		);

		// Assert
		assert_eq!(call.to(), ctf);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	fn bytes(hex: &str) -> Bytes {
		let decoded = alloy_primitives::hex::decode(hex).expect("valid calldata fixture");

		Bytes::from(decoded)
	}
}
