use alloc::vec;

use alloy_primitives::{Address, U256};
use bon::Builder;

use crate::{Call, NonEmptyCalls, erc20, erc1155};

pub fn approve_collateral_for_ctf(collateral_token: Address, ctf: Address, amount: U256) -> Call {
	erc20::approve(collateral_token, ctf, amount)
}

pub fn approve_collateral_for_exchange(
	collateral_token: Address,
	ctf_exchange: Address,
	amount: U256,
) -> Call {
	erc20::approve(collateral_token, ctf_exchange, amount)
}

pub fn approve_collateral_for_neg_risk_exchange(
	collateral_token: Address,
	neg_risk_ctf_exchange: Address,
	amount: U256,
) -> Call {
	erc20::approve(collateral_token, neg_risk_ctf_exchange, amount)
}

pub fn approve_collateral_for_neg_risk_adapter(
	collateral_token: Address,
	neg_risk_adapter: Address,
	amount: U256,
) -> Call {
	erc20::approve(collateral_token, neg_risk_adapter, amount)
}

pub fn approve_ctf_for_exchange(ctf: Address, ctf_exchange: Address) -> Call {
	erc1155::set_approval_for_all(ctf, ctf_exchange, true)
}

pub fn approve_ctf_for_neg_risk_exchange(ctf: Address, neg_risk_ctf_exchange: Address) -> Call {
	erc1155::set_approval_for_all(ctf, neg_risk_ctf_exchange, true)
}

pub fn approve_ctf_for_neg_risk_adapter(ctf: Address, neg_risk_adapter: Address) -> Call {
	erc1155::set_approval_for_all(ctf, neg_risk_adapter, true)
}

pub fn all_approvals(contracts: &PolymarketContracts, amount: U256) -> NonEmptyCalls {
	NonEmptyCalls::new(vec![
		approve_collateral_for_ctf(contracts.collateral_token(), contracts.ctf(), amount),
		approve_collateral_for_neg_risk_adapter(
			contracts.collateral_token(),
			contracts.neg_risk_adapter(),
			amount,
		),
		approve_collateral_for_exchange(
			contracts.collateral_token(),
			contracts.ctf_exchange(),
			amount,
		),
		approve_collateral_for_neg_risk_exchange(
			contracts.collateral_token(),
			contracts.neg_risk_ctf_exchange(),
			amount,
		),
		approve_ctf_for_exchange(contracts.ctf(), contracts.ctf_exchange()),
		approve_ctf_for_neg_risk_exchange(contracts.ctf(), contracts.neg_risk_ctf_exchange()),
		approve_ctf_for_neg_risk_adapter(contracts.ctf(), contracts.neg_risk_adapter()),
	])
	.unwrap() // unwrap is safe as the approval bundle is non-empty
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Builder)]
pub struct PolymarketContracts {
	collateral_token: Address,
	ctf: Address,
	ctf_exchange: Address,
	neg_risk_ctf_exchange: Address,
	neg_risk_adapter: Address,
}

impl PolymarketContracts {
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

	pub fn neg_risk_adapter(&self) -> Address {
		self.neg_risk_adapter
	}
}

#[cfg(test)]
mod tests {
	use alloy_primitives::{U256, address};
	use rstest::rstest;

	use super::*;

	const COLLATERAL_TOKEN: Address = address!("0101010101010101010101010101010101010101");
	const CTF: Address = address!("0202020202020202020202020202020202020202");
	const CTF_EXCHANGE: Address = address!("0303030303030303030303030303030303030303");
	const NEG_RISK_CTF_EXCHANGE: Address = address!("0404040404040404040404040404040404040404");
	const NEG_RISK_ADAPTER: Address = address!("0505050505050505050505050505050505050505");
	const APPROVAL_AMOUNT: u64 = 99;
	const FULL_APPROVAL_COUNT: usize = 7;

	fn contracts() -> PolymarketContracts {
		PolymarketContracts::builder()
			.collateral_token(COLLATERAL_TOKEN)
			.ctf(CTF)
			.ctf_exchange(CTF_EXCHANGE)
			.neg_risk_ctf_exchange(NEG_RISK_CTF_EXCHANGE)
			.neg_risk_adapter(NEG_RISK_ADAPTER)
			.build()
	}

	#[rstest]
	#[case::ctf(
		approve_collateral_for_ctf(
			contracts().collateral_token(),
			contracts().ctf(),
			U256::from(APPROVAL_AMOUNT)
		),
		contracts().collateral_token(),
		contracts().ctf()
	)]
	#[case::neg_risk_adapter(
		approve_collateral_for_neg_risk_adapter(
			contracts().collateral_token(),
			contracts().neg_risk_adapter(),
			U256::from(APPROVAL_AMOUNT)
		),
		contracts().collateral_token(),
		contracts().neg_risk_adapter()
	)]
	#[case::exchange(
		approve_collateral_for_exchange(
			contracts().collateral_token(),
			contracts().ctf_exchange(),
			U256::from(APPROVAL_AMOUNT)
		),
		contracts().collateral_token(),
		contracts().ctf_exchange()
	)]
	#[case::neg_risk_exchange(
		approve_collateral_for_neg_risk_exchange(
			contracts().collateral_token(),
			contracts().neg_risk_ctf_exchange(),
			U256::from(APPROVAL_AMOUNT)
		),
		contracts().collateral_token(),
		contracts().neg_risk_ctf_exchange()
	)]
	fn collateral_recipes_use_caller_supplied_addresses(
		#[case] actual: Call,
		#[case] token: Address,
		#[case] spender: Address,
	) {
		// Arrange
		let expected = erc20::approve(token, spender, U256::from(APPROVAL_AMOUNT));

		// Act
		let actual = actual;

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	#[case::exchange(
		approve_ctf_for_exchange(contracts().ctf(), contracts().ctf_exchange()),
		contracts().ctf(),
		contracts().ctf_exchange()
	)]
	#[case::neg_risk_exchange(
		approve_ctf_for_neg_risk_exchange(contracts().ctf(), contracts().neg_risk_ctf_exchange()),
		contracts().ctf(),
		contracts().neg_risk_ctf_exchange()
	)]
	#[case::neg_risk_adapter(
		approve_ctf_for_neg_risk_adapter(contracts().ctf(), contracts().neg_risk_adapter()),
		contracts().ctf(),
		contracts().neg_risk_adapter()
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

	#[test]
	fn all_approvals_returns_fixed_bundle_order() {
		// Arrange
		let contracts = contracts();
		let amount = U256::from(APPROVAL_AMOUNT);

		// Act
		let calls = all_approvals(&contracts, amount);

		// Assert
		let expected = [
			approve_collateral_for_ctf(contracts.collateral_token(), contracts.ctf(), amount),
			approve_collateral_for_neg_risk_adapter(
				contracts.collateral_token(),
				contracts.neg_risk_adapter(),
				amount,
			),
			approve_collateral_for_exchange(
				contracts.collateral_token(),
				contracts.ctf_exchange(),
				amount,
			),
			approve_collateral_for_neg_risk_exchange(
				contracts.collateral_token(),
				contracts.neg_risk_ctf_exchange(),
				amount,
			),
			approve_ctf_for_exchange(contracts.ctf(), contracts.ctf_exchange()),
			approve_ctf_for_neg_risk_exchange(contracts.ctf(), contracts.neg_risk_ctf_exchange()),
			approve_ctf_for_neg_risk_adapter(contracts.ctf(), contracts.neg_risk_adapter()),
		];
		assert_eq!(calls.len().get(), FULL_APPROVAL_COUNT);
		assert_eq!(calls.as_slice(), expected.as_slice());
	}
}
