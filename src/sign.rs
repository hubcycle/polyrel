//! Transaction signing helpers, calldata builders, and wallet derivation.

use std::borrow::Cow;

use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use alloy_signer::Signer;
use alloy_sol_types::{SolCall, SolStruct, sol};

use crate::{
	PolyrelError,
	types::{Config, OperationType, SignatureParams, SubmitRequest, WalletType},
};

const FIELD_TX_FEE: &str = "tx_fee";
const FIELD_GAS_PRICE: &str = "gas_price";
const FIELD_GAS_LIMIT: &str = "gas_limit";
const FIELD_NONCE: &str = "nonce";

sol! {
	/// Gnosis Safe transaction for EIP-712 signing.
	#[derive(Debug)]
	struct SafeTx {
		address to;
		uint256 value;
		bytes data;
		uint8 operation;
		uint256 safeTxGas;
		uint256 baseGas;
		uint256 gasPrice;
		address gasToken;
		address refundReceiver;
		uint256 nonce;
	}

	/// ERC-20 interface (USDC.e).
	interface IERC20 {
		function approve(address spender, uint256 value) external returns (bool);
		function transfer(address to, uint256 value) external returns (bool);
	}

	/// ERC-1155 interface (Conditional Tokens).
	interface IERC1155 {
		function setApprovalForAll(address operator, bool approved) external;
		function safeTransferFrom(
			address from,
			address to,
			uint256 id,
			uint256 value,
			bytes calldata data
		) external;
	}

	/// Gnosis Safe MultiSend.
	interface IMultiSend {
		function multiSend(bytes transactions) external;
	}

	/// Conditional Tokens Framework.
	interface IConditionalTokens {
		function splitPosition(
			address collateralToken,
			bytes32 parentCollectionId,
			bytes32 conditionId,
			uint256[] calldata partition,
			uint256 amount
		) external;

		function mergePositions(
			address collateralToken,
			bytes32 parentCollectionId,
			bytes32 conditionId,
			uint256[] calldata partition,
			uint256 amount
		) external;

		function redeemPositions(
			address collateralToken,
			bytes32 parentCollectionId,
			bytes32 conditionId,
			uint256[] calldata indexSets
		) external;
	}

	/// Neg-Risk Adapter (different redeem interface).
	interface INegRiskAdapter {
		function redeemPositions(
			bytes32 conditionId,
			uint256[] amounts
		) external;
	}
}

sol! {
	/// Safe Factory `CreateProxy` EIP-712 message.
	struct CreateProxy {
		address paymentToken;
		uint256 payment;
		address paymentReceiver;
	}
}

sol! {
	/// Proxy wallet call.
	struct ProxyCall {
		uint8 typeCode;
		address to;
		uint256 value;
		bytes data;
	}

	/// Proxy Wallet Factory `proxy()` function.
	interface IProxyWalletFactory {
		function proxy(ProxyCall[] calls) external payable returns (bytes[]);
	}
}

/// Target contract and encoded calldata pair.
pub type Call = (Address, Bytes);

/// Proxy call type codes.
const PROXY_CALL_TYPE_CALL: u8 = 1;

/// A non-empty collection of proxy calls.
///
/// Constructed via [`NonEmptyProxyCalls::new`], which rejects empty input.
pub struct NonEmptyProxyCalls(Vec<(Address, Bytes)>);

impl NonEmptyProxyCalls {
	/// Create from a non-empty vector of `(target, calldata)` pairs.
	///
	/// Returns `None` if `calls` is empty.
	pub fn new(calls: Vec<(Address, Bytes)>) -> Option<Self> {
		if calls.is_empty() {
			return None;
		}
		Some(Self(calls))
	}
}

/// Encode transactions for the Proxy Wallet Factory `proxy(calls)` function.
pub fn encode_proxy_calls(transactions: NonEmptyProxyCalls) -> Bytes {
	let calls: Vec<ProxyCall> = transactions
		.0
		.iter()
		.map(|(to, data)| ProxyCall {
			typeCode: PROXY_CALL_TYPE_CALL,
			to: *to,
			value: U256::ZERO,
			data: data.to_vec().into(),
		})
		.collect();
	Bytes::from(IProxyWalletFactory::proxyCall { calls }.abi_encode())
}

/// Approve the CTF Exchange to spend USDC.e.
pub fn usdc_approve_exchange(config: &Config, amount: U256) -> Call {
	let calldata = IERC20::approveCall { spender: config.ctf_exchange(), value: amount };
	(config.usdc_e(), Bytes::from(calldata.abi_encode()))
}

/// Approve the Neg-Risk CTF Exchange to spend USDC.e.
pub fn usdc_approve_neg_risk_exchange(config: &Config, amount: U256) -> Call {
	let calldata = IERC20::approveCall { spender: config.neg_risk_ctf_exchange(), value: amount };
	(config.usdc_e(), Bytes::from(calldata.abi_encode()))
}

/// Transfer USDC.e to a recipient.
pub fn usdc_transfer(config: &Config, to: Address, amount: U256) -> Call {
	let calldata = IERC20::transferCall { to, value: amount };
	(config.usdc_e(), Bytes::from(calldata.abi_encode()))
}

/// Approve the CTF Exchange as operator for Conditional Tokens.
pub fn ctf_approve_exchange(config: &Config) -> Call {
	let calldata =
		IERC1155::setApprovalForAllCall { operator: config.ctf_exchange(), approved: true };
	(
		config.conditional_tokens(),
		Bytes::from(calldata.abi_encode()),
	)
}

/// Approve the Neg-Risk CTF Exchange as operator for Conditional Tokens.
pub fn ctf_approve_neg_risk_exchange(config: &Config) -> Call {
	let calldata = IERC1155::setApprovalForAllCall {
		operator: config.neg_risk_ctf_exchange(),
		approved: true,
	};
	(
		config.conditional_tokens(),
		Bytes::from(calldata.abi_encode()),
	)
}

/// Approve the Conditional Tokens contract to spend USDC.e (for split/merge).
pub fn usdc_approve_conditional_tokens(config: &Config, amount: U256) -> Call {
	let calldata = IERC20::approveCall { spender: config.conditional_tokens(), value: amount };
	(config.usdc_e(), Bytes::from(calldata.abi_encode()))
}

/// Approve the Neg-Risk Adapter to spend USDC.e (for neg-risk operations).
pub fn usdc_approve_neg_risk_adapter(config: &Config, amount: U256) -> Call {
	let calldata = IERC20::approveCall { spender: config.neg_risk_adapter(), value: amount };
	(config.usdc_e(), Bytes::from(calldata.abi_encode()))
}

/// Approve the Neg-Risk Adapter as operator for Conditional Tokens.
pub fn ctf_approve_neg_risk_adapter(config: &Config) -> Call {
	let calldata =
		IERC1155::setApprovalForAllCall { operator: config.neg_risk_adapter(), approved: true };
	(
		config.conditional_tokens(),
		Bytes::from(calldata.abi_encode()),
	)
}

/// Transfer a conditional token position (ERC-1155).
pub fn ctf_transfer(
	config: &Config,
	from: Address,
	to: Address,
	token_id: U256,
	amount: U256,
) -> Call {
	let calldata = IERC1155::safeTransferFromCall {
		from,
		to,
		id: token_id,
		value: amount,
		data: Bytes::new(),
	};
	(
		config.conditional_tokens(),
		Bytes::from(calldata.abi_encode()),
	)
}

/// Split a collateral position into conditional outcome tokens.
pub fn ctf_split_position(
	config: &Config,
	condition_id: B256,
	partition: Vec<U256>,
	amount: U256,
) -> Call {
	let calldata = IConditionalTokens::splitPositionCall {
		collateralToken: config.usdc_e(),
		parentCollectionId: B256::ZERO,
		conditionId: condition_id,
		partition,
		amount,
	};
	(
		config.conditional_tokens(),
		Bytes::from(calldata.abi_encode()),
	)
}

/// Merge conditional outcome tokens back into collateral.
pub fn ctf_merge_positions(
	config: &Config,
	condition_id: B256,
	partition: Vec<U256>,
	amount: U256,
) -> Call {
	let calldata = IConditionalTokens::mergePositionsCall {
		collateralToken: config.usdc_e(),
		parentCollectionId: B256::ZERO,
		conditionId: condition_id,
		partition,
		amount,
	};
	(
		config.conditional_tokens(),
		Bytes::from(calldata.abi_encode()),
	)
}

/// Redeem resolved outcome tokens for collateral.
pub fn ctf_redeem_positions(config: &Config, condition_id: B256, index_sets: Vec<U256>) -> Call {
	let calldata = IConditionalTokens::redeemPositionsCall {
		collateralToken: config.usdc_e(),
		parentCollectionId: B256::ZERO,
		conditionId: condition_id,
		indexSets: index_sets,
	};
	(
		config.conditional_tokens(),
		Bytes::from(calldata.abi_encode()),
	)
}

/// Redeem positions via the Neg-Risk Adapter.
pub fn neg_risk_redeem_positions(condition_id: B256, amounts: Vec<U256>) -> Bytes {
	Bytes::from(
		INegRiskAdapter::redeemPositionsCall { conditionId: condition_id, amounts }.abi_encode(),
	)
}

/// A single Safe transaction within a MultiSend batch.
#[derive(Debug, Clone)]
pub struct SafeTransaction {
	to: Address,
	value: U256,
	data: Vec<u8>,
	operation: OperationType,
}

#[bon::bon]
impl SafeTransaction {
	/// Build a new Safe transaction. `value` defaults to zero and `operation` to `Call`.
	#[builder]
	pub fn new(
		to: Address,
		value: Option<U256>,
		data: Vec<u8>,
		operation: Option<OperationType>,
	) -> Self {
		Self {
			to,
			value: value.unwrap_or(U256::ZERO),
			data,
			operation: operation.unwrap_or(OperationType::Call),
		}
	}

	/// Target contract.
	pub fn to(&self) -> Address {
		self.to
	}

	/// ETH value (usually zero for relayed transactions).
	pub fn value(&self) -> U256 {
		self.value
	}

	/// Encoded calldata.
	pub fn data(&self) -> &[u8] {
		&self.data
	}

	/// Operation type.
	pub fn operation(&self) -> OperationType {
		self.operation
	}
}

/// A non-empty collection of Safe transactions.
///
/// Constructed via [`NonEmptyTransactions::new`], which rejects empty input.
pub struct NonEmptyTransactions(Vec<SafeTransaction>);

impl NonEmptyTransactions {
	/// Create from a non-empty vector.
	pub fn new(transactions: Vec<SafeTransaction>) -> Result<Self, PolyrelError> {
		if transactions.is_empty() {
			return Err(PolyrelError::EmptyBatch);
		}
		Ok(Self(transactions))
	}

	/// Consume into the inner vector.
	pub fn into_inner(self) -> Vec<SafeTransaction> {
		self.0
	}
}

/// Aggregate a batch of Safe transactions into one.
///
/// Returns a single transaction directly when only one is provided.
/// Otherwise encodes as a MultiSend delegate call.
pub fn aggregate_transactions(
	transactions: NonEmptyTransactions,
	multisend_address: Address,
) -> SafeTransaction {
	let txns = transactions.into_inner();
	if txns.len() == 1 {
		return txns.into_iter().next().expect("non-empty");
	}

	let encoded = encode_multisend_payload(&txns);
	let calldata = IMultiSend::multiSendCall { transactions: encoded.into() }.abi_encode();

	SafeTransaction::builder()
		.to(multisend_address)
		.data(calldata)
		.operation(OperationType::DelegateCall)
		.build()
}

/// Encode transactions for the MultiSend contract.
fn encode_multisend_payload(transactions: &[SafeTransaction]) -> Vec<u8> {
	let mut encoded = Vec::new();
	for tx in transactions {
		encoded.push(tx.operation().as_u8());
		encoded.extend_from_slice(tx.to().as_slice());
		encoded.extend_from_slice(&tx.value().to_be_bytes::<32>());
		encoded.extend_from_slice(&U256::from(tx.data().len()).to_be_bytes::<32>());
		encoded.extend_from_slice(tx.data());
	}
	encoded
}

/// Derive the deterministic Safe wallet address for an owner.
pub fn derive_safe_address(owner: Address, safe_factory: Address, init_code_hash: B256) -> Address {
	let encoded = {
		let mut buf = [0u8; 32];
		buf[12..].copy_from_slice(owner.as_slice());
		buf
	};
	let salt = keccak256(encoded);
	create2_address(safe_factory, salt, init_code_hash)
}

/// Derive the deterministic Proxy wallet address for an owner.
pub fn derive_proxy_address(
	owner: Address,
	proxy_factory: Address,
	init_code_hash: B256,
) -> Address {
	let salt = keccak256(owner.as_slice());
	create2_address(proxy_factory, salt, init_code_hash)
}

/// Compute a CREATE2 address.
fn create2_address(deployer: Address, salt: B256, init_code_hash: B256) -> Address {
	let mut buf = [0u8; 1 + 20 + 32 + 32];
	buf[0] = 0xff;
	buf[1..21].copy_from_slice(deployer.as_slice());
	buf[21..53].copy_from_slice(salt.as_slice());
	buf[53..85].copy_from_slice(init_code_hash.as_slice());
	let hash = keccak256(buf);
	Address::from_slice(&hash[12..])
}

/// Compute the EIP-712 Safe transaction hash.
///
/// The returned hash must be signed with `signer.sign_message(hash.as_slice())`
/// (EIP-191 personal_sign), **not** `sign_hash`. The resulting signature must
/// then be packed via [`pack_safe_signature`] before submission. For most
/// use cases, prefer [`crate::RelayerClient::sign_and_submit_safe`] which handles
/// this sequence automatically.
pub fn safe_tx_hash(
	chain_id: u64,
	safe_address: Address,
	tx: &SafeTransaction,
	nonce: U256,
) -> B256 {
	let domain = alloy_sol_types::Eip712Domain {
		chain_id: Some(U256::from(chain_id)),
		verifying_contract: Some(safe_address),
		..Default::default()
	};
	let safe_tx = SafeTx {
		to: tx.to(),
		value: tx.value(),
		data: tx.data().to_vec().into(),
		operation: tx.operation().as_u8(),
		safeTxGas: U256::ZERO,
		baseGas: U256::ZERO,
		gasPrice: U256::ZERO,
		gasToken: Address::ZERO,
		refundReceiver: Address::ZERO,
		nonce,
	};
	safe_tx.eip712_signing_hash(&domain)
}

/// Sign a Safe transaction and return a ready-to-submit [`SubmitRequest`].
///
/// The Safe address is derived deterministically from signer + factory.
pub(crate) async fn sign_safe_transaction<S: Signer + Sync>(
	signer: &S,
	config: &Config,
	tx: SafeTransaction,
	nonce: U256,
) -> Result<SubmitRequest, PolyrelError> {
	let safe_address = derive_safe_address(
		signer.address(),
		config.safe_factory(),
		config.safe_init_code_hash(),
	);
	let signing_hash = safe_tx_hash(config.chain_id(), safe_address, &tx, nonce);

	let signature = signer
		.sign_message(signing_hash.as_slice())
		.await
		.map_err(|e| PolyrelError::signing(e.to_string()))?;

	let packed = pack_safe_signature(&alloy_primitives::hex::encode(signature.as_bytes()))?;

	Ok(SubmitRequest::builder()
		.wallet_type(WalletType::Safe)
		.from(signer.address().to_string().into())
		.to(tx.to().to_string().into())
		.maybe_proxy_wallet(Some(format!("{safe_address:#x}").into()))
		.data(format!("0x{}", alloy_primitives::hex::encode(tx.data())).into())
		.maybe_nonce(Some(nonce.to_string().into()))
		.signature(packed.into())
		.signature_params(SignatureParams::safe(tx.operation().as_u8()))
		.build())
}

/// Sign and build a Safe-create (deployment) request.
///
/// Signs the `CreateProxy` EIP-712 typed data and derives the Safe
/// address deterministically from signer + factory.
pub(crate) async fn sign_safe_create_request<S: Signer + Sync>(
	signer: &S,
	config: &Config,
) -> Result<SubmitRequest, PolyrelError> {
	let safe_address = derive_safe_address(
		signer.address(),
		config.safe_factory(),
		config.safe_init_code_hash(),
	);

	let domain = alloy_sol_types::Eip712Domain {
		name: Some(crate::SAFE_FACTORY_NAME.into()),
		chain_id: Some(U256::from(config.chain_id())),
		verifying_contract: Some(config.safe_factory()),
		..Default::default()
	};
	let msg = CreateProxy {
		paymentToken: Address::ZERO,
		payment: U256::ZERO,
		paymentReceiver: Address::ZERO,
	};
	let hash = msg.eip712_signing_hash(&domain);

	let signature =
		signer.sign_hash(&hash).await.map_err(|e| PolyrelError::signing(e.to_string()))?;

	Ok(SubmitRequest::builder()
		.wallet_type(WalletType::SafeCreate)
		.from(signer.address().to_string().into())
		.to(config.safe_factory().to_string().into())
		.maybe_proxy_wallet(Some(format!("{safe_address:#x}").into()))
		.data("0x".into())
		.signature(format!("0x{}", alloy_primitives::hex::encode(signature.as_bytes())).into())
		.signature_params(SignatureParams::safe_create())
		.build())
}

/// Parameters for a Proxy wallet relay transaction.
pub struct ProxyTransactionArgs {
	data: Bytes,
	nonce: Cow<'static, str>,
	gas_price: Cow<'static, str>,
	gas_limit: Cow<'static, str>,
	relay_address: Address,
}

#[bon::bon]
impl ProxyTransactionArgs {
	/// Build new proxy transaction arguments.
	#[builder]
	pub fn new(
		data: Bytes,
		nonce: Cow<'static, str>,
		gas_price: Cow<'static, str>,
		gas_limit: Cow<'static, str>,
		relay_address: Address,
	) -> Self {
		Self { data, nonce, gas_price, gas_limit, relay_address }
	}

	/// ABI-encoded calldata (e.g., from [`encode_proxy_calls`]).
	pub fn data(&self) -> &Bytes {
		&self.data
	}

	/// Transaction nonce.
	pub fn nonce(&self) -> &str {
		&self.nonce
	}

	/// Gas price.
	pub fn gas_price(&self) -> &str {
		&self.gas_price
	}

	/// Gas limit.
	pub fn gas_limit(&self) -> &str {
		&self.gas_limit
	}

	/// Relay worker address.
	pub fn relay_address(&self) -> Address {
		self.relay_address
	}
}

/// Sign a Proxy wallet transaction and return a ready-to-submit [`SubmitRequest`].
pub async fn sign_proxy_transaction<S: Signer + Sync>(
	signer: &S,
	config: &Config,
	args: ProxyTransactionArgs,
) -> Result<SubmitRequest, PolyrelError> {
	let gas_limit = parse_u256(&args.gas_limit, FIELD_GAS_LIMIT)?;
	if gas_limit.is_zero() {
		return Err(PolyrelError::InvalidNumericField {
			field: FIELD_GAS_LIMIT,
			value: Cow::Borrowed("0"),
		});
	}
	let factory = config.proxy_wallet_factory();
	let proxy_address =
		derive_proxy_address(signer.address(), factory, config.proxy_init_code_hash());
	let struct_hash = proxy_struct_hash(
		signer.address(),
		factory,
		&args.data,
		"0",
		&args.gas_price,
		&args.gas_limit,
		&args.nonce,
		config.relay_hub(),
		args.relay_address,
	)?;

	let signature = signer
		.sign_message(struct_hash.as_slice())
		.await
		.map_err(|e| PolyrelError::signing(e.to_string()))?;

	Ok(SubmitRequest::builder()
		.wallet_type(WalletType::Proxy)
		.from(signer.address().to_string().into())
		.to(factory.to_string().into())
		.maybe_proxy_wallet(Some(proxy_address.to_string().into()))
		.data(format!("0x{}", alloy_primitives::hex::encode(&args.data)).into())
		.maybe_nonce(Some(args.nonce))
		.signature(format!("0x{}", alloy_primitives::hex::encode(signature.as_bytes())).into())
		.signature_params(SignatureParams::proxy(
			args.gas_price,
			args.gas_limit,
			config.relay_hub(),
			args.relay_address,
		))
		.build())
}

/// Pack a signature into Safe's expected format.
///
/// Adjusts the v byte: `0/1 → +31`, `27/28 → +4`.
pub fn pack_safe_signature(sig_hex: &str) -> Result<String, PolyrelError> {
	let raw = sig_hex.strip_prefix("0x").unwrap_or(sig_hex);
	let mut bytes = alloy_primitives::hex::decode(raw)
		.map_err(|_| PolyrelError::invalid_signature("hex decode failed"))?;

	const EXPECTED_LEN: usize = 65;
	if bytes.len() != EXPECTED_LEN {
		return Err(PolyrelError::invalid_signature(
			"signature must be 65 bytes",
		));
	}

	bytes[64] = match bytes[64] {
		0 | 1 => bytes[64] + 31,
		27 | 28 => bytes[64] + 4,
		_ => {
			return Err(PolyrelError::invalid_signature(
				"invalid v value in signature",
			));
		},
	};

	Ok(format!("0x{}", alloy_primitives::hex::encode(bytes)))
}

/// Build a Proxy-wallet struct hash for signing.
///
/// Concatenates: `"rlx:" ++ from ++ to ++ data ++ txFee(32) ++ gasPrice(32)
/// ++ gasLimit(32) ++ nonce(32) ++ relayHub ++ relay`.
fn parse_u256(value: &str, field: &'static str) -> Result<U256, PolyrelError> {
	value.parse::<U256>().map_err(|_| PolyrelError::InvalidNumericField {
		field,
		value: Cow::Owned(value.to_owned()),
	})
}

#[allow(clippy::too_many_arguments)]
fn proxy_struct_hash(
	from: Address,
	to: Address,
	data: &[u8],
	tx_fee: &str,
	gas_price: &str,
	gas_limit: &str,
	nonce: &str,
	relay_hub: Address,
	relay_address: Address,
) -> Result<B256, PolyrelError> {
	let prefix = b"rlx:";
	let tx_fee_u256 = parse_u256(tx_fee, FIELD_TX_FEE)?;
	let gas_price_u256 = parse_u256(gas_price, FIELD_GAS_PRICE)?;
	let gas_limit_u256 = parse_u256(gas_limit, FIELD_GAS_LIMIT)?;
	let nonce_u256 = parse_u256(nonce, FIELD_NONCE)?;

	let mut buf = Vec::new();
	buf.extend_from_slice(prefix);
	buf.extend_from_slice(from.as_slice());
	buf.extend_from_slice(to.as_slice());
	buf.extend_from_slice(data);
	buf.extend_from_slice(&tx_fee_u256.to_be_bytes::<32>());
	buf.extend_from_slice(&gas_price_u256.to_be_bytes::<32>());
	buf.extend_from_slice(&gas_limit_u256.to_be_bytes::<32>());
	buf.extend_from_slice(&nonce_u256.to_be_bytes::<32>());
	buf.extend_from_slice(relay_hub.as_slice());
	buf.extend_from_slice(relay_address.as_slice());
	Ok(keccak256(&buf))
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[case(0, 31)]
	#[case(1, 32)]
	#[case(27, 31)]
	#[case(28, 32)]
	fn pack_adjusts_v_byte(#[case] input_v: u8, #[case] expected: u8) {
		// Arrange
		let mut sig = vec![0xaa; 64];
		sig.push(input_v);
		let hex = alloy_primitives::hex::encode(&sig);

		// Act
		let packed = pack_safe_signature(&hex).unwrap();

		// Assert
		let bytes = alloy_primitives::hex::decode(packed.strip_prefix("0x").unwrap()).unwrap();
		assert_eq!(bytes[64], expected);
	}

	#[test]
	fn pack_rejects_invalid_v() {
		// Arrange
		let mut sig = vec![0xee; 64];
		sig.push(5);
		let hex = alloy_primitives::hex::encode(&sig);

		// Act
		let result = pack_safe_signature(&hex);

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn pack_rejects_wrong_length() {
		// Arrange
		let hex = alloy_primitives::hex::encode(vec![0u8; 60]);

		// Act
		let result = pack_safe_signature(&hex);

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn pack_handles_0x_prefix() {
		// Arrange
		let mut sig = vec![0xaa; 64];
		sig.push(0);
		let hex = format!("0x{}", alloy_primitives::hex::encode(&sig));

		// Act
		let packed = pack_safe_signature(&hex);

		// Assert
		assert!(packed.is_ok());
	}

	#[test]
	fn proxy_struct_hash_rejects_invalid_nonce() {
		// Act
		let result = proxy_struct_hash(
			Address::ZERO,
			Address::ZERO,
			&[],
			"0",
			"0",
			"1000000",
			"not_a_number",
			Address::ZERO,
			Address::ZERO,
		);

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn proxy_struct_hash_rejects_invalid_gas_limit() {
		// Act
		let result = proxy_struct_hash(
			Address::ZERO,
			Address::ZERO,
			&[],
			"0",
			"0",
			"abc",
			"0",
			Address::ZERO,
			Address::ZERO,
		);

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn proxy_struct_hash_accepts_valid_inputs() {
		// Act
		let result = proxy_struct_hash(
			Address::ZERO,
			Address::ZERO,
			&[0xde, 0xad],
			"0",
			"0",
			"10000000",
			"42",
			Address::ZERO,
			Address::ZERO,
		);

		// Assert
		assert!(result.is_ok());
	}

	#[test]
	fn non_empty_transactions_rejects_empty() {
		// Act
		let result = NonEmptyTransactions::new(vec![]);

		// Assert
		assert!(result.is_err());
	}

	#[test]
	fn non_empty_transactions_accepts_one() {
		// Arrange
		let tx = SafeTransaction::builder().to(Address::ZERO).data(vec![]).build();

		// Act
		let result = NonEmptyTransactions::new(vec![tx]);

		// Assert
		assert!(result.is_ok());
	}

	#[test]
	fn non_empty_proxy_calls_rejects_empty() {
		// Act
		let result = NonEmptyProxyCalls::new(vec![]);

		// Assert
		assert!(result.is_none());
	}

	#[test]
	fn non_empty_proxy_calls_accepts_one() {
		// Arrange
		let call = (Address::ZERO, Bytes::new());

		// Act
		let result = NonEmptyProxyCalls::new(vec![call]);

		// Assert
		assert!(result.is_some());
	}

	#[test]
	fn derive_safe_address_is_deterministic() {
		// Arrange
		let owner = Address::ZERO;
		let factory = crate::SAFE_FACTORY;
		let hash = crate::SAFE_INIT_CODE_HASH.into();

		// Act
		let a = derive_safe_address(owner, factory, hash);
		let b = derive_safe_address(owner, factory, hash);

		// Assert
		assert_eq!(a, b);
		assert_ne!(a, Address::ZERO);
	}

	#[test]
	fn derive_proxy_address_is_deterministic() {
		// Arrange
		let owner = Address::ZERO;
		let factory = crate::PROXY_WALLET_FACTORY;
		let hash = crate::PROXY_INIT_CODE_HASH.into();

		// Act
		let a = derive_proxy_address(owner, factory, hash);
		let b = derive_proxy_address(owner, factory, hash);

		// Assert
		assert_eq!(a, b);
		assert_ne!(a, Address::ZERO);
	}

	#[test]
	fn different_init_code_hash_yields_different_address() {
		// Arrange
		let owner = Address::ZERO;
		let factory = crate::SAFE_FACTORY;
		let default_hash: B256 = crate::SAFE_INIT_CODE_HASH.into();
		let custom_hash = B256::ZERO;

		// Act
		let default_addr = derive_safe_address(owner, factory, default_hash);
		let custom_addr = derive_safe_address(owner, factory, custom_hash);

		// Assert
		assert_ne!(default_addr, custom_addr);
	}

	#[test]
	fn usdc_approve_calldata_starts_with_approve_selector() {
		// Arrange
		let config = crate::types::Config::builder().build().unwrap();
		let approve_selector: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];

		// Act
		let (_, data) = usdc_approve_exchange(&config, U256::from(100));

		// Assert
		assert_eq!(&data[..4], &approve_selector);
	}

	#[test]
	fn ctf_approve_calldata_starts_with_set_approval_selector() {
		// Arrange
		let config = crate::types::Config::builder().build().unwrap();
		let set_approval_selector: [u8; 4] = [0xa2, 0x2c, 0xb4, 0x65];

		// Act
		let (_, data) = ctf_approve_exchange(&config);

		// Assert
		assert_eq!(&data[..4], &set_approval_selector);
	}

	#[test]
	fn usdc_approve_conditional_tokens_encodes_correct_spender_and_amount() {
		// Arrange
		let config = crate::types::Config::builder().build().unwrap();
		let amount = U256::from(100);

		// Act
		let (target, data) = usdc_approve_conditional_tokens(&config, amount);

		// Assert: target is USDC.e
		assert_eq!(target, config.usdc_e());
		// Assert: selector is approve(address,uint256)
		assert_eq!(&data[..4], &[0x09, 0x5e, 0xa7, 0xb3]);
		// Assert: spender is conditional_tokens (bytes 4..36, left-padded address)
		let spender = Address::from_slice(&data[16..36]);
		assert_eq!(spender, config.conditional_tokens());
		// Assert: amount is 100 (bytes 36..68)
		let encoded_amount = U256::from_be_slice(&data[36..68]);
		assert_eq!(encoded_amount, amount);
	}

	#[test]
	fn usdc_approve_neg_risk_adapter_encodes_correct_spender_and_amount() {
		// Arrange
		let config = crate::types::Config::builder().build().unwrap();
		let amount = U256::from(42);

		// Act
		let (target, data) = usdc_approve_neg_risk_adapter(&config, amount);

		// Assert: target is USDC.e
		assert_eq!(target, config.usdc_e());
		// Assert: selector is approve(address,uint256)
		assert_eq!(&data[..4], &[0x09, 0x5e, 0xa7, 0xb3]);
		// Assert: spender is neg_risk_adapter
		let spender = Address::from_slice(&data[16..36]);
		assert_eq!(spender, config.neg_risk_adapter());
		// Assert: amount is 42
		let encoded_amount = U256::from_be_slice(&data[36..68]);
		assert_eq!(encoded_amount, amount);
	}

	#[test]
	fn ctf_approve_neg_risk_adapter_encodes_correct_operator_and_approved() {
		// Arrange
		let config = crate::types::Config::builder().build().unwrap();

		// Act
		let (target, data) = ctf_approve_neg_risk_adapter(&config);

		// Assert: target is conditional_tokens
		assert_eq!(target, config.conditional_tokens());
		// Assert: selector is setApprovalForAll(address,bool)
		assert_eq!(&data[..4], &[0xa2, 0x2c, 0xb4, 0x65]);
		// Assert: operator is neg_risk_adapter
		let operator = Address::from_slice(&data[16..36]);
		assert_eq!(operator, config.neg_risk_adapter());
		// Assert: approved is true (bytes 36..68, last byte is 1)
		assert_eq!(data[67], 1);
	}

	#[test]
	fn aggregate_single_returns_directly() {
		// Arrange
		let tx = SafeTransaction::builder().to(Address::ZERO).data(vec![0xde, 0xad]).build();
		let batch = NonEmptyTransactions::new(vec![tx]).unwrap();

		// Act
		let result = aggregate_transactions(batch, crate::SAFE_MULTISEND);

		// Assert
		assert_eq!(result.operation(), OperationType::Call);
		assert_eq!(result.data(), &[0xde, 0xad]);
	}

	#[test]
	fn aggregate_multiple_produces_delegate_call() {
		// Arrange
		let tx1 = SafeTransaction::builder().to(Address::ZERO).data(vec![0x01]).build();
		let tx2 = SafeTransaction::builder().to(Address::ZERO).data(vec![0x02]).build();
		let batch = NonEmptyTransactions::new(vec![tx1, tx2]).unwrap();

		// Act
		let result = aggregate_transactions(batch, crate::SAFE_MULTISEND);

		// Assert
		assert_eq!(result.operation(), OperationType::DelegateCall);
		assert_eq!(result.to(), crate::SAFE_MULTISEND);
	}

	#[tokio::test]
	async fn sign_proxy_transaction_rejects_zero_gas_limit() {
		// Arrange
		let signer = alloy_signer_local::PrivateKeySigner::random();
		let config = crate::types::Config::builder().build().unwrap();
		let args = ProxyTransactionArgs::builder()
			.data(Bytes::from(vec![0xde, 0xad]))
			.nonce("1".into())
			.gas_price("0".into())
			.gas_limit("0".into())
			.relay_address(Address::ZERO)
			.build();

		// Act
		let result = sign_proxy_transaction(&signer, &config, args).await;

		// Assert
		assert!(matches!(
			result,
			Err(PolyrelError::InvalidNumericField { field: FIELD_GAS_LIMIT, .. })
		));
	}

	#[tokio::test]
	async fn sign_safe_create_request_produces_correct_fields() {
		// Arrange
		let signer = alloy_signer_local::PrivateKeySigner::random();
		let config = crate::types::Config::builder().build().unwrap();

		// Act
		let request = sign_safe_create_request(&signer, &config).await.unwrap();

		// Assert
		let expected_safe = derive_safe_address(
			signer.address(),
			config.safe_factory(),
			config.safe_init_code_hash(),
		);
		assert_eq!(request.wallet_type, WalletType::SafeCreate);
		assert_eq!(request.data, "0x");
		assert_eq!(request.to, config.safe_factory().to_string());
		assert_eq!(request.from, signer.address().to_string());
		assert_eq!(
			request.proxy_wallet.as_deref(),
			Some(format!("{expected_safe:#x}").as_str())
		);
		assert!(request.signature.starts_with("0x"));
		assert!(request.nonce.is_none());

		let zero = Address::ZERO.to_string();
		let params = &request.signature_params;
		assert_eq!(params.payment_token.as_deref(), Some(zero.as_str()));
		assert_eq!(params.payment.as_deref(), Some("0"));
		assert_eq!(params.payment_receiver.as_deref(), Some(zero.as_str()));
		assert!(params.gas_price.is_none());
	}
}
