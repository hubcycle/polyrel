//! Polymarket V2 collateral helpers for pUSD wrap, unwrap, and CTF adapter flows.
//!
//! # Example
//!
//! ```no_run
//! use alloy_primitives::{U256, address, b256};
//! use polyrel::collateral;
//!
//! let _wrap = collateral::wrap(
//!     address!("93070a847efEf7F70739046A929D47a521F5B8ee"),
//!     address!("2791Bca1f2de4661ED88A30C99A7a9449Aa84174"),
//!     address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5"),
//!     U256::from(1_000_000_u64),
//! );
//!
//! let _split = collateral::split_position(
//!     address!("ADa100874d00e3331D00F2007a9c336a65009718"),
//!     b256!("edcac9eebf21d7bf4ab3f0ceda931b194c67fa3192d6d3d3b7b74b0983e5bff1"),
//!     U256::from(1_000_000_u64),
//! );
//! ```

use alloc::vec;

use alloy_primitives::{Address, B256, Bytes, U256};
use alloy_sol_types::{SolCall, sol};

use crate::Call;

const ZERO_ADDRESS: Address = Address::ZERO;
const ZERO_BYTES32: B256 = B256::ZERO;
const PARTITION_NO: u8 = 2;
const PARTITION_YES: u8 = 1;

sol! {
	interface ICollateralOnramp {
		function wrap(address _asset, address _to, uint256 _amount) external;
	}

	interface ICollateralOfframp {
		function unwrap(address _asset, address _to, uint256 _amount) external;
	}

	interface ICtfCollateralAdapter {
		function splitPosition(
			address,
			bytes32,
			bytes32 _conditionId,
			uint256[] partition,
			uint256 _amount
		) external;

		function mergePositions(
			address,
			bytes32,
			bytes32 _conditionId,
			uint256[] partition,
			uint256 _amount
		) external;

		function redeemPositions(
			address,
			bytes32,
			bytes32 _conditionId,
			uint256[] indexSets
		) external;
	}
}

/// Builds `CollateralOnramp.wrap(asset, to, amount)` calldata.
pub fn wrap(onramp: Address, asset: Address, to: Address, amount: U256) -> Call {
	let data = Bytes::from(
		ICollateralOnramp::wrapCall { _asset: asset, _to: to, _amount: amount }.abi_encode(),
	);

	Call::builder().to(onramp).data(data).build()
}

/// Builds `CollateralOfframp.unwrap(asset, to, amount)` calldata.
pub fn unwrap(offramp: Address, asset: Address, to: Address, amount: U256) -> Call {
	let data = Bytes::from(
		ICollateralOfframp::unwrapCall { _asset: asset, _to: to, _amount: amount }.abi_encode(),
	);

	Call::builder().to(offramp).data(data).build()
}

/// Builds `CtfCollateralAdapter.splitPosition(...)` calldata for a binary market.
pub fn split_position(adapter: Address, condition_id: B256, amount: U256) -> Call {
	let data = Bytes::from(
		ICtfCollateralAdapter::splitPositionCall {
			_0: ZERO_ADDRESS,
			_1: ZERO_BYTES32,
			_conditionId: condition_id,
			partition: binary_partition(),
			_amount: amount,
		}
		.abi_encode(),
	);

	Call::builder().to(adapter).data(data).build()
}

/// Builds `CtfCollateralAdapter.mergePositions(...)` calldata for a binary market.
pub fn merge_positions(adapter: Address, condition_id: B256, amount: U256) -> Call {
	let data = Bytes::from(
		ICtfCollateralAdapter::mergePositionsCall {
			_0: ZERO_ADDRESS,
			_1: ZERO_BYTES32,
			_conditionId: condition_id,
			partition: binary_partition(),
			_amount: amount,
		}
		.abi_encode(),
	);

	Call::builder().to(adapter).data(data).build()
}

/// Builds `CtfCollateralAdapter.redeemPositions(...)` calldata for a binary market.
pub fn redeem_positions(adapter: Address, condition_id: B256) -> Call {
	let data = Bytes::from(
		ICtfCollateralAdapter::redeemPositionsCall {
			_0: ZERO_ADDRESS,
			_1: ZERO_BYTES32,
			_conditionId: condition_id,
			indexSets: binary_partition(),
		}
		.abi_encode(),
	);

	Call::builder().to(adapter).data(data).build()
}

fn binary_partition() -> alloc::vec::Vec<U256> {
	vec![U256::from(PARTITION_YES), U256::from(PARTITION_NO)]
}

#[cfg(test)]
mod tests {
	use alloy_primitives::{Bytes, U256, address, b256};

	use super::*;

	const ONRAMP: Address = address!("1111111111111111111111111111111111111111");
	const OFFRAMP: Address = address!("2222222222222222222222222222222222222222");
	const ADAPTER: Address = address!("3333333333333333333333333333333333333333");
	const ASSET: Address = address!("4444444444444444444444444444444444444444");
	const RECIPIENT: Address = address!("5555555555555555555555555555555555555555");
	const CONDITION_ID: B256 =
		b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
	const AMOUNT: u64 = 42;
	const WRAP_EXPECTED: &str = "6235563800000000000000000000000044444444444444444444444444444444444444440000000000000000000000005555555555555555555555555555555555555555000000000000000000000000000000000000000000000000000000000000002a";
	const UNWRAP_EXPECTED: &str = "8cc7104f00000000000000000000000044444444444444444444444444444444444444440000000000000000000000005555555555555555555555555555555555555555000000000000000000000000000000000000000000000000000000000000002a";
	const SPLIT_EXPECTED: &str = "72ce427500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa00000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000002a000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002";
	const MERGE_EXPECTED: &str = "9e7212ad00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa00000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000002a000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002";
	const REDEEM_EXPECTED: &str = "01b7037c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002";
	const VALID_CALLDATA_FIXTURE: &str = "valid calldata fixture";

	#[test]
	fn wrap_encodes_expected_calldata() {
		// Arrange
		let expected = bytes(WRAP_EXPECTED);

		// Act
		let call = wrap(ONRAMP, ASSET, RECIPIENT, U256::from(AMOUNT));

		// Assert
		assert_eq!(call.to(), ONRAMP);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	#[test]
	fn unwrap_encodes_expected_calldata() {
		// Arrange
		let expected = bytes(UNWRAP_EXPECTED);

		// Act
		let call = unwrap(OFFRAMP, ASSET, RECIPIENT, U256::from(AMOUNT));

		// Assert
		assert_eq!(call.to(), OFFRAMP);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	#[test]
	fn split_position_encodes_expected_calldata() {
		// Arrange
		let expected = bytes(SPLIT_EXPECTED);

		// Act
		let call = split_position(ADAPTER, CONDITION_ID, U256::from(AMOUNT));

		// Assert
		assert_eq!(call.to(), ADAPTER);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	#[test]
	fn merge_positions_encodes_expected_calldata() {
		// Arrange
		let expected = bytes(MERGE_EXPECTED);

		// Act
		let call = merge_positions(ADAPTER, CONDITION_ID, U256::from(AMOUNT));

		// Assert
		assert_eq!(call.to(), ADAPTER);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	#[test]
	fn redeem_positions_encodes_expected_calldata() {
		// Arrange
		let expected = bytes(REDEEM_EXPECTED);

		// Act
		let call = redeem_positions(ADAPTER, CONDITION_ID);

		// Assert
		assert_eq!(call.to(), ADAPTER);
		assert_eq!(call.value(), U256::ZERO);
		assert_eq!(call.data(), &expected);
	}

	fn bytes(hex: &str) -> Bytes {
		let decoded = alloy_primitives::hex::decode(hex).expect(VALID_CALLDATA_FIXTURE);

		Bytes::from(decoded)
	}
}
