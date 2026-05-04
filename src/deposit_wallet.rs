//! Polymarket deposit-wallet relayer builders.
//!
//! This module covers the relayer-side deposit-wallet flow:
//!
//! - derive the deterministic deposit wallet address
//! - build a `WALLET-CREATE` relayer request
//! - build and sign a `WALLET` batch relayer request
//!
//! It does not implement CLOB `POLY_1271` order signing.
//!
//! # Examples
//!
//! ```no_run
//! use alloy_primitives::{Address, U256, address};
//! use polyrel::{
//!     NonEmptyCalls, deposit_wallet, erc20,
//!     safe::ChainId,
//! };
//!
//! let create_context = deposit_wallet::DepositWalletCreateContext::builder()
//!     .owner(address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5"))
//!     .deposit_wallet_factory(address!("00000000000fb5c9adea0298d729a0cb3823cc07"))
//!     .deposit_wallet_implementation(address!("50a88fe9a441cb4c9c2ad6a2207ce2795c7d7fbd"))
//!     .build();
//! let create_draft = deposit_wallet::build_create_draft(&create_context);
//! let deposit_wallet = create_draft.deposit_wallet_address();
//!
//! let approve = erc20::approve(
//!     address!("c011a7e12a19f7b1f670d46f03b03f3342e82dfb"),
//!     address!("4d97dcd97ec945f40cf65f87097ace5ea0476045"),
//!     U256::MAX,
//! );
//! let batch_context = deposit_wallet::DepositWalletBatchContext::builder()
//!     .owner(address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5"))
//!     .chain_id(ChainId::new(137.try_into().unwrap()))
//!     .deposit_wallet_factory(address!("00000000000fb5c9adea0298d729a0cb3823cc07"))
//!     .deposit_wallet(deposit_wallet)
//!     .nonce(deposit_wallet::DepositWalletNonce::new(U256::ZERO))
//!     .deadline(deposit_wallet::DepositWalletDeadline::new(U256::from(1_760_000_000_u64)))
//!     .build();
//! let _batch_draft =
//!     deposit_wallet::build_batch_draft(&batch_context, NonEmptyCalls::from_one(approve))
//!         .unwrap();
//! ```

use alloc::{
	collections::BTreeMap,
	string::{String, ToString},
	vec,
	vec::Vec,
};

use alloy_primitives::{Address, B256, Signature, U256, b256};
use alloy_sol_types::{Eip712Domain, SolStruct, SolValue};
use bon::Builder;
use serde::Serialize;

use crate::{
	NonEmptyCalls, PolyrelError,
	safe::{ChainId, SubmitKind, TypedDataDomainJson, TypedDataFieldJson, TypedDataJson},
};

const DOMAIN_TYPE: &str = "EIP712Domain";
const FIELD_NAME_CALLS: &str = "calls";
const FIELD_NAME_CHAIN_ID: &str = "chainId";
const FIELD_NAME_DATA: &str = "data";
const FIELD_NAME_DEADLINE: &str = "deadline";
const FIELD_NAME_NAME: &str = "name";
const FIELD_NAME_NONCE: &str = "nonce";
const FIELD_NAME_TARGET: &str = "target";
const FIELD_NAME_VALUE: &str = "value";
const FIELD_NAME_VERIFYING_CONTRACT: &str = "verifyingContract";
const FIELD_NAME_VERSION: &str = "version";
const FIELD_NAME_WALLET: &str = "wallet";
const EIP712_DOMAIN_NAME: &str = "DepositWallet";
const EIP712_DOMAIN_VERSION: &str = "1";
const ERC1967_CONST1: B256 =
	b256!("cc3735a920a3ca505d382bbc545af43d6000803e6038573d6000fd5b3d6000f3");
const ERC1967_CONST2: B256 =
	b256!("5155f3363d3d373d3d363d7f360894a13ba1a3210667c828492db98dca3e2076");
const ERC1967_PREFIX: u128 = 0x61003d3d8160233d3973;
const ERC1967_SUFFIX: [u8; 2] = [0x60, 0x09];
const PRIMARY_TYPE_BATCH: &str = "Batch";
const PRIMARY_TYPE_CALL: &str = "Call";
const SOL_TYPE_ADDRESS: &str = "address";
const SOL_TYPE_BYTES: &str = "bytes";
const SOL_TYPE_CALL_ARRAY: &str = "Call[]";
const SOL_TYPE_STRING: &str = "string";
const SOL_TYPE_UINT256: &str = "uint256";

alloy_sol_types::sol! {
	struct Call {
		address target;
		uint256 value;
		bytes data;
	}

	struct Batch {
		address wallet;
		uint256 nonce;
		uint256 deadline;
		Call[] calls;
	}

	struct WalletArgs {
		address factory;
		bytes32 walletId;
	}
}

/// Deposit-wallet batches reuse the crate-wide generic call envelope.
pub type DepositWalletCall = CallEnvelope;

type CallEnvelope = crate::Call;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Deposit-wallet relayer nonce.
pub struct DepositWalletNonce(U256);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Deposit-wallet relayer batch deadline.
pub struct DepositWalletDeadline(U256);

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
/// Context required to derive a deposit wallet address and build `WALLET-CREATE`.
pub struct DepositWalletCreateContext {
	owner: Address,
	deposit_wallet_factory: Address,
	deposit_wallet_implementation: Address,
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
/// Context required to build a signed `WALLET` batch.
pub struct DepositWalletBatchContext {
	owner: Address,
	chain_id: ChainId,
	deposit_wallet_factory: Address,
	deposit_wallet: Address,
	nonce: DepositWalletNonce,
	deadline: DepositWalletDeadline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// JSON-serializable deposit-wallet `Call` typed-data message.
pub struct DepositWalletCallJson {
	/// Batch target address.
	pub target: String,
	/// Native token value as a decimal string.
	pub value: String,
	/// Calldata as a hex string.
	pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// JSON-serializable deposit-wallet `Batch` typed-data message.
pub struct DepositWalletBatchMessageJson {
	/// Deposit wallet address.
	pub wallet: String,
	/// Batch nonce as a decimal string.
	pub nonce: String,
	/// Batch deadline as a decimal string.
	pub deadline: String,
	/// Calls included in the batch.
	pub calls: Vec<DepositWalletCallJson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// `depositWalletParams` payload accepted by the relayer for `WALLET` requests.
pub struct DepositWalletParams {
	/// Deposit wallet address.
	pub deposit_wallet: String,
	/// Batch deadline as a decimal string.
	pub deadline: String,
	/// Calls to execute from the deposit wallet.
	pub calls: Vec<DepositWalletCallJson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Typed relayer request for `WALLET-CREATE`.
pub struct DepositWalletCreateRequest {
	/// Request sender address.
	pub from: String,
	/// Deposit wallet factory address.
	pub to: String,
	#[serde(rename = "type")]
	/// Relayer transaction type.
	pub kind: SubmitKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// Typed relayer request for `WALLET`.
pub struct DepositWalletBatchRequest {
	/// Request sender address.
	pub from: String,
	/// Deposit wallet factory address.
	pub to: String,
	/// Current `WALLET` nonce as a decimal string.
	pub nonce: String,
	/// Normal 65-byte EIP-712 signature as a hex string.
	pub signature: String,
	#[serde(rename = "type")]
	/// Relayer transaction type.
	pub kind: SubmitKind,
	/// Wallet-batch execution parameters.
	pub deposit_wallet_params: DepositWalletParams,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Unsigned `WALLET-CREATE` draft.
pub struct DepositWalletCreateDraft {
	deposit_wallet_address: Address,
	request: DepositWalletCreateRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Unsigned `WALLET` batch draft.
pub struct DepositWalletBatchDraft {
	deposit_wallet_address: Address,
	signing_hash: B256,
	typed_data: TypedDataJson<DepositWalletBatchMessageJson>,
	request_base: DepositWalletBatchRequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DepositWalletBatchRequestBase {
	from: Address,
	to: Address,
	nonce: DepositWalletNonce,
	deposit_wallet_params: DepositWalletParams,
}

impl DepositWalletNonce {
	/// Creates a deposit-wallet nonce from the raw value.
	pub fn new(raw: U256) -> Self {
		Self(raw)
	}

	/// Returns the underlying nonce value.
	pub fn raw(&self) -> U256 {
		self.0
	}
}

impl DepositWalletDeadline {
	/// Creates a deposit-wallet deadline from the raw value.
	pub fn new(raw: U256) -> Self {
		Self(raw)
	}

	/// Returns the underlying deadline value.
	pub fn raw(&self) -> U256 {
		self.0
	}
}

impl DepositWalletCreateContext {
	/// Returns the owner address used for wallet creation and batch signing.
	pub fn owner(&self) -> Address {
		self.owner
	}

	/// Returns the deposit wallet factory address.
	pub fn deposit_wallet_factory(&self) -> Address {
		self.deposit_wallet_factory
	}

	/// Returns the deposit-wallet implementation address used for ERC-1967 bytecode derivation.
	pub fn deposit_wallet_implementation(&self) -> Address {
		self.deposit_wallet_implementation
	}
}

impl DepositWalletBatchContext {
	/// Returns the owner address used to sign the batch.
	pub fn owner(&self) -> Address {
		self.owner
	}

	/// Returns the chain ID used in the EIP-712 domain.
	pub fn chain_id(&self) -> ChainId {
		self.chain_id
	}

	/// Returns the deposit wallet factory address.
	pub fn deposit_wallet_factory(&self) -> Address {
		self.deposit_wallet_factory
	}

	/// Returns the deposit wallet address.
	pub fn deposit_wallet(&self) -> Address {
		self.deposit_wallet
	}

	/// Returns the current relayer nonce.
	pub fn nonce(&self) -> DepositWalletNonce {
		self.nonce
	}

	/// Returns the batch deadline.
	pub fn deadline(&self) -> DepositWalletDeadline {
		self.deadline
	}
}

impl DepositWalletCreateDraft {
	/// Returns the derived deposit wallet address.
	pub fn deposit_wallet_address(&self) -> Address {
		self.deposit_wallet_address
	}

	/// Returns the relayer request for `WALLET-CREATE`.
	pub fn request(&self) -> &DepositWalletCreateRequest {
		&self.request
	}

	/// Consumes the draft and returns the relayer request.
	pub fn into_submit_request(self) -> DepositWalletCreateRequest {
		self.request
	}
}

impl DepositWalletBatchDraft {
	/// Domain type name used for deposit-wallet batch typed data.
	pub const DOMAIN_TYPE: &str = DOMAIN_TYPE;
	/// Primary type name used for deposit-wallet batch typed data.
	pub const PRIMARY_TYPE: &str = PRIMARY_TYPE_BATCH;

	/// Returns the deposit wallet address.
	pub fn deposit_wallet_address(&self) -> Address {
		self.deposit_wallet_address
	}

	/// Returns the EIP-712 signing hash for the batch.
	pub fn signing_hash(&self) -> B256 {
		self.signing_hash
	}

	/// Returns the JSON-serializable typed-data payload for signing.
	pub fn typed_data(&self) -> &TypedDataJson<DepositWalletBatchMessageJson> {
		&self.typed_data
	}

	/// Finalizes the draft into a relayer `WALLET` request using a normal 65-byte signature.
	pub fn into_submit_request(self, signature: Signature) -> DepositWalletBatchRequest {
		self.request_base.into_submit_request(signature)
	}
}

/// Derives the deterministic deposit wallet address from the owner, factory, and implementation.
pub fn derive_address(
	owner: Address,
	deposit_wallet_factory: Address,
	deposit_wallet_implementation: Address,
) -> Address {
	let salt = derive_salt(owner, deposit_wallet_factory);
	let bytecode_hash =
		init_code_hash_erc1967(deposit_wallet_implementation, owner, deposit_wallet_factory);

	create2_address(deposit_wallet_factory, salt, bytecode_hash)
}

/// Builds an unsigned `WALLET-CREATE` draft.
pub fn build_create_draft(context: &DepositWalletCreateContext) -> DepositWalletCreateDraft {
	let deposit_wallet_address = derive_address(
		context.owner(),
		context.deposit_wallet_factory(),
		context.deposit_wallet_implementation(),
	);

	DepositWalletCreateDraft {
		deposit_wallet_address,
		request: DepositWalletCreateRequest {
			from: address_string(context.owner()),
			to: address_string(context.deposit_wallet_factory()),
			kind: SubmitKind::WalletCreate,
		},
	}
}

/// Builds an unsigned `WALLET` batch draft for one or more deposit-wallet calls.
pub fn build_batch_draft(
	context: &DepositWalletBatchContext,
	calls: NonEmptyCalls,
) -> Result<DepositWalletBatchDraft, PolyrelError> {
	let batch_calls = sol_batch_calls(calls.as_slice());
	let domain = Eip712Domain {
		name: Some(EIP712_DOMAIN_NAME.to_string().into()),
		version: Some(EIP712_DOMAIN_VERSION.to_string().into()),
		chain_id: Some(U256::from(context.chain_id().raw())),
		verifying_contract: Some(context.deposit_wallet()),
		..Default::default()
	};
	let batch = Batch {
		wallet: context.deposit_wallet(),
		nonce: context.nonce().raw(),
		deadline: context.deadline().raw(),
		calls: batch_calls,
	};
	let signing_hash = batch.eip712_signing_hash(&domain);
	let deposit_wallet_params = DepositWalletParams {
		deposit_wallet: address_string(context.deposit_wallet()),
		deadline: context.deadline().raw().to_string(),
		calls: json_calls(calls.as_slice()),
	};
	let typed_data = batch_typed_data_json(context, &deposit_wallet_params.calls);

	Ok(DepositWalletBatchDraft {
		deposit_wallet_address: context.deposit_wallet(),
		signing_hash,
		typed_data,
		request_base: DepositWalletBatchRequestBase {
			from: context.owner(),
			to: context.deposit_wallet_factory(),
			nonce: context.nonce(),
			deposit_wallet_params,
		},
	})
}

impl DepositWalletBatchRequestBase {
	fn into_submit_request(self, signature: Signature) -> DepositWalletBatchRequest {
		DepositWalletBatchRequest {
			from: address_string(self.from),
			to: address_string(self.to),
			nonce: self.nonce.raw().to_string(),
			signature: hex_string(signature.as_bytes().as_ref()),
			kind: SubmitKind::Wallet,
			deposit_wallet_params: self.deposit_wallet_params,
		}
	}
}

fn batch_typed_data_json(
	context: &DepositWalletBatchContext,
	calls: &[DepositWalletCallJson],
) -> TypedDataJson<DepositWalletBatchMessageJson> {
	let mut types = BTreeMap::new();
	types.insert(DOMAIN_TYPE.to_string(), eip712_domain_fields());
	types.insert(PRIMARY_TYPE_CALL.to_string(), batch_call_fields());
	types.insert(PRIMARY_TYPE_BATCH.to_string(), batch_fields());

	TypedDataJson {
		types,
		primary_type: PRIMARY_TYPE_BATCH.to_string(),
		domain: TypedDataDomainJson {
			name: Some(EIP712_DOMAIN_NAME.to_string()),
			version: Some(EIP712_DOMAIN_VERSION.to_string()),
			chain_id: Some(context.chain_id().raw()),
			verifying_contract: Some(address_string(context.deposit_wallet())),
			salt: None,
		},
		message: DepositWalletBatchMessageJson {
			wallet: address_string(context.deposit_wallet()),
			nonce: context.nonce().raw().to_string(),
			deadline: context.deadline().raw().to_string(),
			calls: calls.to_vec(),
		},
	}
}

fn eip712_domain_fields() -> Vec<TypedDataFieldJson> {
	vec![
		TypedDataFieldJson { name: FIELD_NAME_NAME, type_name: SOL_TYPE_STRING },
		TypedDataFieldJson { name: FIELD_NAME_VERSION, type_name: SOL_TYPE_STRING },
		TypedDataFieldJson { name: FIELD_NAME_CHAIN_ID, type_name: SOL_TYPE_UINT256 },
		TypedDataFieldJson { name: FIELD_NAME_VERIFYING_CONTRACT, type_name: SOL_TYPE_ADDRESS },
	]
}

fn batch_call_fields() -> Vec<TypedDataFieldJson> {
	vec![
		TypedDataFieldJson { name: FIELD_NAME_TARGET, type_name: SOL_TYPE_ADDRESS },
		TypedDataFieldJson { name: FIELD_NAME_VALUE, type_name: SOL_TYPE_UINT256 },
		TypedDataFieldJson { name: FIELD_NAME_DATA, type_name: SOL_TYPE_BYTES },
	]
}

fn batch_fields() -> Vec<TypedDataFieldJson> {
	vec![
		TypedDataFieldJson { name: FIELD_NAME_WALLET, type_name: SOL_TYPE_ADDRESS },
		TypedDataFieldJson { name: FIELD_NAME_NONCE, type_name: SOL_TYPE_UINT256 },
		TypedDataFieldJson { name: FIELD_NAME_DEADLINE, type_name: SOL_TYPE_UINT256 },
		TypedDataFieldJson { name: FIELD_NAME_CALLS, type_name: SOL_TYPE_CALL_ARRAY },
	]
}

fn json_calls(calls: &[DepositWalletCall]) -> Vec<DepositWalletCallJson> {
	calls
		.iter()
		.map(|call| DepositWalletCallJson {
			target: address_string(call.to()),
			value: call.value().to_string(),
			data: hex_string(call.data().as_ref()),
		})
		.collect()
}

fn sol_batch_calls(calls: &[DepositWalletCall]) -> Vec<Call> {
	calls
		.iter()
		.map(|call| Call { target: call.to(), value: call.value(), data: call.data().clone() })
		.collect()
}

fn derive_salt(owner: Address, deposit_wallet_factory: Address) -> B256 {
	let encoded = wallet_args(owner, deposit_wallet_factory).abi_encode();

	alloy_primitives::keccak256(encoded)
}

fn wallet_args(owner: Address, deposit_wallet_factory: Address) -> WalletArgs {
	WalletArgs { factory: deposit_wallet_factory, walletId: owner_word(owner) }
}

fn owner_word(owner: Address) -> B256 {
	let mut encoded = [0_u8; 32];
	encoded[12..].copy_from_slice(owner.as_slice());

	B256::from(encoded)
}

fn init_code_hash_erc1967(
	deposit_wallet_implementation: Address,
	owner: Address,
	deposit_wallet_factory: Address,
) -> B256 {
	let args = wallet_args(owner, deposit_wallet_factory).abi_encode();
	let combined = ERC1967_PREFIX + ((args.len() as u128) << 56);
	let combined_bytes = combined.to_be_bytes();
	let prefix_bytes = &combined_bytes[combined_bytes.len() - 10..];
	let mut init_code = Vec::with_capacity(10 + 20 + 2 + 32 + 32 + args.len());

	init_code.extend_from_slice(prefix_bytes);
	init_code.extend_from_slice(deposit_wallet_implementation.as_slice());
	init_code.extend_from_slice(&ERC1967_SUFFIX);
	init_code.extend_from_slice(ERC1967_CONST2.as_slice());
	init_code.extend_from_slice(ERC1967_CONST1.as_slice());
	init_code.extend_from_slice(&args);

	alloy_primitives::keccak256(init_code)
}

fn create2_address(deployer: Address, salt: B256, init_code_hash: B256) -> Address {
	let mut payload = [0_u8; 85];
	payload[0] = 0xff;
	payload[1..21].copy_from_slice(deployer.as_slice());
	payload[21..53].copy_from_slice(salt.as_slice());
	payload[53..85].copy_from_slice(init_code_hash.as_slice());
	let hash = alloy_primitives::keccak256(payload);

	Address::from_slice(&hash[12..])
}

fn address_string(address: Address) -> String {
	address.to_string()
}

fn hex_string(bytes: &[u8]) -> String {
	alloy_primitives::hex::encode_prefixed(bytes)
}

#[cfg(test)]
mod tests {
	use alloc::vec;

	use alloy_primitives::{Bytes, U256, address, b256};

	use super::*;
	use crate::{erc20, erc1155};

	const TEST_BATCH_HASH: B256 =
		b256!("877466d6bf84de12b6298ac32dc37e596977d8e2539a177fb6f8513faf1b2e42");
	const TEST_BATCH_TYPED_DATA_JSON: &str = r#"{"types":{"Batch":[{"name":"wallet","type":"address"},{"name":"nonce","type":"uint256"},{"name":"deadline","type":"uint256"},{"name":"calls","type":"Call[]"}],"Call":[{"name":"target","type":"address"},{"name":"value","type":"uint256"},{"name":"data","type":"bytes"}],"EIP712Domain":[{"name":"name","type":"string"},{"name":"version","type":"string"},{"name":"chainId","type":"uint256"},{"name":"verifyingContract","type":"address"}]},"primaryType":"Batch","domain":{"name":"DepositWallet","version":"1","chainId":137,"verifyingContract":"0x7777777777777777777777777777777777777777"},"message":{"wallet":"0x7777777777777777777777777777777777777777","nonce":"9","deadline":"1760000000","calls":[{"target":"0x1111111111111111111111111111111111111111","value":"0","data":"0xdeadbeef"},{"target":"0x2222222222222222222222222222222222222222","value":"3","data":"0xcafe"}]}}"#;
	const TEST_CREATE_REQUEST_JSON: &str = r#"{"from":"0x6e0c80c90ea6c15917308F820Eac91Ce2724B5b5","to":"0x00000000000Fb5C9ADea0298D729A0CB3823Cc07","type":"WALLET-CREATE"}"#;
	const TEST_DEADLINE: u64 = 1_760_000_000;
	const TEST_DEPOSIT_WALLET: Address = address!("7777777777777777777777777777777777777777");
	const TEST_DEPOSIT_WALLET_IMPLEMENTATION: Address =
		address!("50a88fe9a441cb4c9c2ad6a2207ce2795c7d7fbd");
	const TEST_DERIVED_ADDRESS: Address = address!("02d39815f255814edb3cbb59aee668878cd95ac3");
	const TEST_FACTORY: Address = address!("00000000000fb5c9adea0298d729a0cb3823cc07");
	const TEST_FIRST_CALL_TARGET: Address = address!("1111111111111111111111111111111111111111");
	const TEST_NONCE: u64 = 9;
	const TEST_OWNER: Address = address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5");
	const TEST_SECOND_CALL_TARGET: Address = address!("2222222222222222222222222222222222222222");
	const TEST_SIGNATURE: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb1b";
	const TEST_WALLET_REQUEST_JSON: &str = r#"{"from":"0x6e0c80c90ea6c15917308F820Eac91Ce2724B5b5","to":"0x00000000000Fb5C9ADea0298D729A0CB3823Cc07","nonce":"9","signature":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb1b","type":"WALLET","depositWalletParams":{"depositWallet":"0x7777777777777777777777777777777777777777","deadline":"1760000000","calls":[{"target":"0x1111111111111111111111111111111111111111","value":"0","data":"0xdeadbeef"},{"target":"0x2222222222222222222222222222222222222222","value":"3","data":"0xcafe"}]}}"#;

	fn create_context() -> DepositWalletCreateContext {
		DepositWalletCreateContext::builder()
			.owner(TEST_OWNER)
			.deposit_wallet_factory(TEST_FACTORY)
			.deposit_wallet_implementation(TEST_DEPOSIT_WALLET_IMPLEMENTATION)
			.build()
	}

	fn batch_context() -> DepositWalletBatchContext {
		DepositWalletBatchContext::builder()
			.owner(TEST_OWNER)
			.chain_id(ChainId::new(137.try_into().unwrap()))
			.deposit_wallet_factory(TEST_FACTORY)
			.deposit_wallet(TEST_DEPOSIT_WALLET)
			.nonce(DepositWalletNonce::new(U256::from(TEST_NONCE)))
			.deadline(DepositWalletDeadline::new(U256::from(TEST_DEADLINE)))
			.build()
	}

	fn fixture_calls() -> NonEmptyCalls {
		NonEmptyCalls::new(vec![
			CallEnvelope::builder()
				.to(TEST_FIRST_CALL_TARGET)
				.data(Bytes::from_static(&[0xde, 0xad, 0xbe, 0xef]))
				.build(),
			CallEnvelope::builder()
				.to(TEST_SECOND_CALL_TARGET)
				.data(Bytes::from_static(&[0xca, 0xfe]))
				.value(U256::from(3_u8))
				.build(),
		])
		.unwrap()
	}

	fn wallet_signature() -> Signature {
		let mut bytes = [0xbb; 65];
		bytes[64] = 27;

		Signature::from_raw_array(&bytes).unwrap()
	}

	#[test]
	fn derive_address_matches_expected_fixture() {
		// Arrange
		let context = create_context();

		// Act
		let address = derive_address(
			context.owner(),
			context.deposit_wallet_factory(),
			context.deposit_wallet_implementation(),
		);

		// Assert
		assert_eq!(address, TEST_DERIVED_ADDRESS);
	}

	#[test]
	fn create_draft_serializes_expected_submit_request() {
		// Arrange
		let draft = build_create_draft(&create_context());

		// Act
		let json = serde_json::to_string(draft.request()).unwrap();

		// Assert
		assert_eq!(draft.deposit_wallet_address(), TEST_DERIVED_ADDRESS);
		assert_eq!(json, TEST_CREATE_REQUEST_JSON);
	}

	#[test]
	fn batch_draft_serializes_expected_typed_data() {
		// Arrange
		let draft = build_batch_draft(&batch_context(), fixture_calls()).unwrap();

		// Act
		let json = serde_json::to_string(draft.typed_data()).unwrap();

		// Assert
		assert_eq!(draft.signing_hash(), TEST_BATCH_HASH);
		assert_eq!(json, TEST_BATCH_TYPED_DATA_JSON);
	}

	#[test]
	fn batch_draft_serializes_expected_submit_request() {
		// Arrange
		let request = build_batch_draft(&batch_context(), fixture_calls())
			.unwrap()
			.into_submit_request(wallet_signature());

		// Act
		let json = serde_json::to_string(&request).unwrap();

		// Assert
		assert_eq!(json, TEST_WALLET_REQUEST_JSON);
		assert_eq!(request.signature, TEST_SIGNATURE);
	}

	#[test]
	fn batch_draft_accepts_existing_calldata_helpers_for_approval_recipes() {
		// Arrange
		let pusd = address!("c011a7e12a19f7b1f670d46f03b03f3342e82dfb");
		let ctf = address!("4d97dcd97ec945f40cf65f87097ace5ea0476045");
		let exchange = address!("e111180000d2663c0091e4f400237545b87b996b");
		let calls = NonEmptyCalls::new(vec![
			erc20::approve(pusd, ctf, U256::MAX),
			erc1155::set_approval_for_all(ctf, exchange, true),
		])
		.unwrap();

		// Act
		let draft = build_batch_draft(&batch_context(), calls).unwrap();

		// Assert
		assert_eq!(draft.typed_data().message.calls.len(), 2);
		assert_eq!(draft.typed_data().message.calls[0].target, pusd.to_string());
		assert_eq!(draft.typed_data().message.calls[1].target, ctf.to_string());
	}
}
