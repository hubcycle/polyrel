use alloy_primitives::{Address, U256};

use crate::{Call, erc20, erc1155};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolymarketContracts {
	collateral_token: Address,
	ctf: Address,
	ctf_exchange: Address,
	neg_risk_ctf_exchange: Address,
}

#[bon::bon]
impl PolymarketContracts {
	#[builder]
	pub fn new(
		collateral_token: Address,
		ctf: Address,
		ctf_exchange: Address,
		neg_risk_ctf_exchange: Address,
	) -> Self {
		Self { collateral_token, ctf, ctf_exchange, neg_risk_ctf_exchange }
	}

	pub fn collateral_token(&self) -> Address {
		self.collateral_token
	}

	pub fn ctf(&self) -> Address {
		self.ctf
	}

	pub fn ctf_exchange(&self) -> Address {
		self.ctf_exchange
	}

	pub fn neg_risk_ctf_exchange(&self) -> Address {
		self.neg_risk_ctf_exchange
	}
}

pub fn approve_collateral_for_ctf(contracts: &PolymarketContracts, amount: U256) -> Call {
	erc20::approve(contracts.collateral_token(), contracts.ctf(), amount)
}

pub fn approve_collateral_for_exchange(contracts: &PolymarketContracts, amount: U256) -> Call {
	erc20::approve(
		contracts.collateral_token(),
		contracts.ctf_exchange(),
		amount,
	)
}

pub fn approve_collateral_for_neg_risk_exchange(
	contracts: &PolymarketContracts,
	amount: U256,
) -> Call {
	erc20::approve(
		contracts.collateral_token(),
		contracts.neg_risk_ctf_exchange(),
		amount,
	)
}

pub fn approve_ctf_for_exchange(contracts: &PolymarketContracts) -> Call {
	erc1155::set_approval_for_all(contracts.ctf(), contracts.ctf_exchange(), true)
}

pub fn approve_ctf_for_neg_risk_exchange(contracts: &PolymarketContracts) -> Call {
	erc1155::set_approval_for_all(contracts.ctf(), contracts.neg_risk_ctf_exchange(), true)
}

#[cfg(test)]
mod tests {
	use alloy_primitives::{U256, address};
	use rstest::rstest;

	use super::*;

	fn contracts() -> PolymarketContracts {
		PolymarketContracts::builder()
			.collateral_token(address!("0101010101010101010101010101010101010101"))
			.ctf(address!("0202020202020202020202020202020202020202"))
			.ctf_exchange(address!("0303030303030303030303030303030303030303"))
			.neg_risk_ctf_exchange(address!("0404040404040404040404040404040404040404"))
			.build()
	}

	#[rstest]
	#[case::ctf(approve_collateral_for_ctf(&contracts(), U256::from(99_u64)), contracts().collateral_token(), contracts().ctf())]
	#[case::exchange(
		approve_collateral_for_exchange(&contracts(), U256::from(99_u64)),
		contracts().collateral_token(),
		contracts().ctf_exchange()
	)]
	#[case::neg_risk_exchange(
		approve_collateral_for_neg_risk_exchange(&contracts(), U256::from(99_u64)),
		contracts().collateral_token(),
		contracts().neg_risk_ctf_exchange()
	)]
	fn collateral_recipes_use_caller_supplied_addresses(
		#[case] actual: Call,
		#[case] token: Address,
		#[case] spender: Address,
	) {
		// Arrange
		let expected = erc20::approve(token, spender, U256::from(99_u64));

		// Act
		let actual = actual;

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	#[case::exchange(approve_ctf_for_exchange(&contracts()), contracts().ctf(), contracts().ctf_exchange())]
	#[case::neg_risk_exchange(
		approve_ctf_for_neg_risk_exchange(&contracts()),
		contracts().ctf(),
		contracts().neg_risk_ctf_exchange()
	)]
	fn ctf_operator_recipes_use_caller_supplied_addresses(
		#[case] actual: Call,
		#[case] token: Address,
		#[case] operator: Address,
	) {
		// Arrange
		let expected = erc1155::set_approval_for_all(token, operator, true);

		// Act
		let actual = actual;

		// Assert
		assert_eq!(actual, expected);
	}
}
