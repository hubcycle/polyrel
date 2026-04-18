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

	const SPLIT_CTF: Address = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
	const SPLIT_COLLATERAL_TOKEN: Address = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
	const SPLIT_PARENT_COLLECTION_ID: B256 = B256::ZERO;
	const SPLIT_CONDITION_ID: B256 =
		b256!("1111111111111111111111111111111111111111111111111111111111111111");
	const SPLIT_PARTITION_ONE: u64 = 1;
	const SPLIT_PARTITION_TWO: u64 = 2;
	const SPLIT_AMOUNT: u64 = 10;
	const SPLIT_EXPECTED: &str = "72ce4275000000000000000000000000bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb0000000000000000000000000000000000000000000000000000000000000000111111111111111111111111111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002";
	const MERGE_CTF: Address = address!("cccccccccccccccccccccccccccccccccccccccc");
	const MERGE_COLLATERAL_TOKEN: Address = address!("dddddddddddddddddddddddddddddddddddddddd");
	const MERGE_PARENT_COLLECTION_ID: B256 =
		b256!("2222222222222222222222222222222222222222222222222222222222222222");
	const MERGE_CONDITION_ID: B256 =
		b256!("3333333333333333333333333333333333333333333333333333333333333333");
	const MERGE_PARTITION_ONE: u64 = 4;
	const MERGE_PARTITION_TWO: u64 = 8;
	const MERGE_AMOUNT: u64 = 25;
	const MERGE_EXPECTED: &str = "9e7212ad000000000000000000000000dddddddddddddddddddddddddddddddddddddddd2222222222222222222222222222222222222222222222222222222222222222333333333333333333333333333333333333333333333333333333333333333300000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000019000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008";
	const REDEEM_CTF: Address = address!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
	const REDEEM_COLLATERAL_TOKEN: Address = address!("ffffffffffffffffffffffffffffffffffffffff");
	const REDEEM_PARENT_COLLECTION_ID: B256 =
		b256!("4444444444444444444444444444444444444444444444444444444444444444");
	const REDEEM_CONDITION_ID: B256 =
		b256!("5555555555555555555555555555555555555555555555555555555555555555");
	const REDEEM_INDEX_SET_ONE: u64 = 1;
	const REDEEM_INDEX_SET_TWO: u64 = 3;
	const REDEEM_EXPECTED: &str = "01b7037c000000000000000000000000ffffffffffffffffffffffffffffffffffffffff444444444444444444444444444444444444444444444444444444444444444455555555555555555555555555555555555555555555555555555555555555550000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000003";
	const VALID_CALLDATA_FIXTURE: &str = "valid calldata fixture";

	#[test]
	fn split_position_encodes_expected_calldata() {
		// Arrange
		let ctf = SPLIT_CTF;
		let collateral_token = SPLIT_COLLATERAL_TOKEN;
		let parent_collection_id = SPLIT_PARENT_COLLECTION_ID;
		let condition_id = SPLIT_CONDITION_ID;
		let partition = vec![
			U256::from(SPLIT_PARTITION_ONE),
			U256::from(SPLIT_PARTITION_TWO),
		];
		let amount = U256::from(SPLIT_AMOUNT);
		let expected = bytes(SPLIT_EXPECTED);

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
		let ctf = MERGE_CTF;
		let collateral_token = MERGE_COLLATERAL_TOKEN;
		let parent_collection_id = MERGE_PARENT_COLLECTION_ID;
		let condition_id = MERGE_CONDITION_ID;
		let partition = vec![
			U256::from(MERGE_PARTITION_ONE),
			U256::from(MERGE_PARTITION_TWO),
		];
		let amount = U256::from(MERGE_AMOUNT);
		let expected = bytes(MERGE_EXPECTED);

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
		let ctf = REDEEM_CTF;
		let collateral_token = REDEEM_COLLATERAL_TOKEN;
		let parent_collection_id = REDEEM_PARENT_COLLECTION_ID;
		let condition_id = REDEEM_CONDITION_ID;
		let index_sets = vec![
			U256::from(REDEEM_INDEX_SET_ONE),
			U256::from(REDEEM_INDEX_SET_TWO),
		];
		let expected = bytes(REDEEM_EXPECTED);

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
		let decoded = alloy_primitives::hex::decode(hex).expect(VALID_CALLDATA_FIXTURE);

		Bytes::from(decoded)
	}
}
