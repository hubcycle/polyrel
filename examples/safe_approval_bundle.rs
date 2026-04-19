//! Example showing how to build a bundled Safe approval request.

use alloy_primitives::{Address, B256, Signature, U256};

use polyrel::{
	NonEmptyCalls,
	polymarket::{self, PolymarketContracts},
	safe::{
		self, ChainId, Metadata, PackedSafeSignature, SafeExecutionContext, SafeGasParams,
		SafeNonce,
	},
};

const COLLATERAL_TOKEN: Address =
	alloy_primitives::address!("c011a7e12a19f7b1f670d46f03b03f3342e82dfb");

const CTF: Address = alloy_primitives::address!("4d97dcd97ec945f40cf65f87097ace5ea0476045");

const CTF_EXCHANGE: Address =
	alloy_primitives::address!("e111dc5a03edd8f0f4f1d0f7d5c2811e202a2f7f");

const NEG_RISK_CTF_EXCHANGE: Address =
	alloy_primitives::address!("e222dc5a03edd8f0f4f1d0f7d5c2811e202a2f7f");

const NEG_RISK_ADAPTER: Address =
	alloy_primitives::address!("d91e80cf2e7be2e162c6513ced06f1dd0da35296");

const OWNER: Address = alloy_primitives::address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5");

const SAFE_FACTORY: Address =
	alloy_primitives::address!("aacfeea03eb1561c4e67d661e40682bd20e3541b");

const SAFE_MULTISEND: Address =
	alloy_primitives::address!("a238cbeb142c10ef7ad8442c6d1f9e89e07e7761");

const SAFE_INIT_CODE_HASH: B256 =
	alloy_primitives::b256!("2bce2127ff07fb632d16c8347c4ebf501f4841168bed00d9e6ef715ddb6fcecf");

const APPROVE_ALL_METADATA: &str = "approve-all";

fn main() {
	let contracts = PolymarketContracts::builder()
		.collateral_token(COLLATERAL_TOKEN)
		.ctf(CTF)
		.ctf_exchange(CTF_EXCHANGE)
		.neg_risk_ctf_exchange(NEG_RISK_CTF_EXCHANGE)
		.neg_risk_adapter(NEG_RISK_ADAPTER)
		.build();

	let calls: NonEmptyCalls = polymarket::all_approvals(&contracts, U256::MAX);
	let context = SafeExecutionContext::builder()
		.owner(OWNER)
		.chain_id(ChainId::new(137.try_into().unwrap()))
		.safe_factory(SAFE_FACTORY)
		.safe_init_code_hash(SAFE_INIT_CODE_HASH)
		.safe_multisend(SAFE_MULTISEND)
		.nonce(SafeNonce::new(U256::from(1_u64)))
		.gas_params(
			SafeGasParams::builder()
				.safe_txn_gas(U256::ZERO)
				.base_gas(U256::ZERO)
				.gas_price(U256::ZERO)
				.gas_token(Address::ZERO)
				.refund_receiver(Address::ZERO)
				.build(),
		)
		.metadata(Metadata::new(APPROVE_ALL_METADATA.into()))
		.build();

	let draft = safe::build_execution_draft(&context, calls).unwrap();

	let wallet_signature = {
		let mut bytes = [0x22; 65];
		bytes[64] = 27;
		Signature::from_raw_array(&bytes).unwrap()
	};

	let request =
		draft.into_submit_request(PackedSafeSignature::from_wallet_signature(wallet_signature));

	println!(
		"Built SAFE submit request for {} targeting {}",
		request.proxy_wallet.unwrap(),
		request.to
	);
}
