use core::num::NonZeroU64;

use alloc::{
	borrow::Cow,
	collections::BTreeMap,
	format,
	string::{String, ToString},
	vec,
	vec::Vec,
};

use alloy_primitives::{Address, B256, Bytes, Signature, U256, keccak256};
use alloy_sol_types::{Eip712Domain, SolCall, SolStruct, sol};
use bon::Builder;
use serde::Serialize;

use crate::{Call, NonEmptyCalls, PolyrelError};

const SOL_TYPE_ADDRESS: &str = "address";
const SOL_TYPE_STRING: &str = "string";
const SOL_TYPE_UINT256: &str = "uint256";
const FIELD_NAME_CHAIN_ID: &str = "chainId";
const FIELD_NAME_NAME: &str = "name";
const FIELD_NAME_PAYMENT: &str = "payment";
const FIELD_NAME_PAYMENT_RECEIVER: &str = "paymentReceiver";
const FIELD_NAME_PAYMENT_TOKEN: &str = "paymentToken";
const FIELD_NAME_VERIFYING_CONTRACT: &str = "verifyingContract";

sol! {
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

	struct CreateProxy {
		address paymentToken;
		uint256 payment;
		address paymentReceiver;
	}

	interface IMultiSend {
		function multiSend(bytes transactions) external;
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafeOperation {
	Call,
	DelegateCall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SubmitKind {
	#[serde(rename = "SAFE")]
	Safe,

	#[serde(rename = "SAFE-CREATE")]
	SafeCreate,

	#[serde(rename = "PROXY")]
	Proxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainId(NonZeroU64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafeNonce(U256);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactoryDomainName(Cow<'static, str>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata(Cow<'static, str>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedSafeSignature([u8; 65]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Builder)]
pub struct SafeCreatePayment {
	payment_token: Address,
	payment: U256,
	payment_receiver: Address,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Builder)]
pub struct SafeGasParams {
	safe_txn_gas: U256,
	base_gas: U256,
	gas_price: U256,
	gas_token: Address,
	refund_receiver: Address,
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct SafeCreateContext {
	owner: Address,
	chain_id: ChainId,
	safe_factory: Address,
	safe_init_code_hash: B256,
	factory_domain_name: FactoryDomainName,
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct SafeExecutionContext {
	owner: Address,
	chain_id: ChainId,
	safe_factory: Address,
	safe_init_code_hash: B256,
	safe_multisend: Address,
	nonce: SafeNonce,
	gas_params: SafeGasParams,
	metadata: Option<Metadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypedDataFieldJson {
	pub name: &'static str,

	#[serde(rename = "type")]
	pub type_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedDataDomainJson {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub version: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub chain_id: Option<u64>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub verifying_contract: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub salt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProxyMessageJson {
	pub payment_token: String,
	pub payment: String,
	pub payment_receiver: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypedDataJson<M>
where
	M: Serialize,
{
	pub types: BTreeMap<String, Vec<TypedDataFieldJson>>,

	#[serde(rename = "primaryType")]
	pub primary_type: String,

	pub domain: TypedDataDomainJson,
	pub message: M,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureParams {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub gas_price: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub operation: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub safe_txn_gas: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub base_gas: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub gas_token: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub refund_receiver: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub payment_token: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub payment: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub payment_receiver: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub relayer_fee: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub gas_limit: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub relay_hub: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub relay: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRequest {
	#[serde(rename = "type")]
	pub kind: SubmitKind,

	pub from: String,
	pub to: String,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub proxy_wallet: Option<String>,

	pub data: String,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub nonce: Option<String>,

	pub signature: String,
	pub signature_params: SignatureParams,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeCreateDraft {
	safe_address: Address,
	signing_hash: B256,
	typed_data: TypedDataJson<CreateProxyMessageJson>,
	submit_base: DraftSubmitBase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeExecutionDraft {
	safe_address: Address,
	aggregated_call: Call,
	operation: SafeOperation,
	signing_hash: B256,
	submit_base: DraftSubmitBase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DraftSubmitBase {
	kind: SubmitKind,
	from: Address,
	to: Address,
	safe_address: Address,
	data: Bytes,
	nonce: Option<SafeNonce>,
	signature_params: SignatureParams,
	metadata: Option<Metadata>,
}

impl ChainId {
	pub fn new(raw: NonZeroU64) -> Self {
		Self(raw)
	}

	pub fn raw(&self) -> u64 {
		self.0.get()
	}
}

impl SafeNonce {
	pub fn new(raw: U256) -> Self {
		Self(raw)
	}

	pub fn raw(&self) -> U256 {
		self.0
	}
}

impl FactoryDomainName {
	pub fn new(value: Cow<'static, str>) -> Result<Self, PolyrelError> {
		if value.is_empty() {
			return Err(PolyrelError::validation(
				"factory domain name must not be empty",
			));
		}

		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		self.0.as_ref()
	}
}

impl Metadata {
	pub fn new(value: Cow<'static, str>) -> Self {
		Self(value)
	}

	pub fn as_str(&self) -> &str {
		self.0.as_ref()
	}
}

impl PackedSafeSignature {
	pub const V_MIN: u8 = 31;
	pub const V_MAX: u8 = 32;

	pub fn new(bytes: [u8; 65]) -> Result<Self, PolyrelError> {
		validate_packed_signature_bytes(&bytes)?;

		Ok(Self(bytes))
	}

	pub fn from_wallet_signature(signature: Signature) -> Self {
		let mut bytes = signature.as_bytes();
		bytes[64] += 4;

		Self(bytes)
	}

	pub fn as_bytes(&self) -> &[u8; 65] {
		&self.0
	}

	pub fn into_bytes(self) -> [u8; 65] {
		self.0
	}
}

impl SafeCreatePayment {
	pub fn payment_token(&self) -> Address {
		self.payment_token
	}

	pub fn payment(&self) -> U256 {
		self.payment
	}

	pub fn payment_receiver(&self) -> Address {
		self.payment_receiver
	}
}

impl SafeGasParams {
	pub fn safe_txn_gas(&self) -> U256 {
		self.safe_txn_gas
	}

	pub fn base_gas(&self) -> U256 {
		self.base_gas
	}

	pub fn gas_price(&self) -> U256 {
		self.gas_price
	}

	pub fn gas_token(&self) -> Address {
		self.gas_token
	}

	pub fn refund_receiver(&self) -> Address {
		self.refund_receiver
	}
}

impl SafeCreateContext {
	pub fn owner(&self) -> Address {
		self.owner
	}

	pub fn chain_id(&self) -> ChainId {
		self.chain_id
	}

	pub fn safe_factory(&self) -> Address {
		self.safe_factory
	}

	pub fn safe_init_code_hash(&self) -> B256 {
		self.safe_init_code_hash
	}

	pub fn factory_domain_name(&self) -> &FactoryDomainName {
		&self.factory_domain_name
	}
}

impl SafeExecutionContext {
	pub fn owner(&self) -> Address {
		self.owner
	}

	pub fn chain_id(&self) -> ChainId {
		self.chain_id
	}

	pub fn safe_factory(&self) -> Address {
		self.safe_factory
	}

	pub fn safe_init_code_hash(&self) -> B256 {
		self.safe_init_code_hash
	}

	pub fn safe_multisend(&self) -> Address {
		self.safe_multisend
	}

	pub fn nonce(&self) -> SafeNonce {
		self.nonce
	}

	pub fn gas_params(&self) -> &SafeGasParams {
		&self.gas_params
	}

	pub fn metadata(&self) -> Option<&Metadata> {
		self.metadata.as_ref()
	}
}

impl SafeOperation {
	pub fn as_u8(self) -> u8 {
		match self {
			Self::Call => 0,
			Self::DelegateCall => 1,
		}
	}
}

impl SignatureParams {
	pub fn safe(operation: SafeOperation, gas_params: &SafeGasParams) -> Self {
		Self {
			gas_price: Some(gas_params.gas_price().to_string()),
			operation: Some(operation.as_u8().to_string()),
			safe_txn_gas: Some(gas_params.safe_txn_gas().to_string()),
			base_gas: Some(gas_params.base_gas().to_string()),
			gas_token: Some(address_string(gas_params.gas_token())),
			refund_receiver: Some(address_string(gas_params.refund_receiver())),
			payment_token: None,
			payment: None,
			payment_receiver: None,
			relayer_fee: None,
			gas_limit: None,
			relay_hub: None,
			relay: None,
		}
	}

	pub fn safe_create(payment: &SafeCreatePayment) -> Self {
		Self {
			gas_price: None,
			operation: None,
			safe_txn_gas: None,
			base_gas: None,
			gas_token: None,
			refund_receiver: None,
			payment_token: Some(address_string(payment.payment_token())),
			payment: Some(payment.payment().to_string()),
			payment_receiver: Some(address_string(payment.payment_receiver())),
			relayer_fee: None,
			gas_limit: None,
			relay_hub: None,
			relay: None,
		}
	}
}

impl SafeCreateDraft {
	pub const PRIMARY_TYPE: &str = "CreateProxy";
	pub const DOMAIN_TYPE: &str = "EIP712Domain";

	pub fn safe_address(&self) -> Address {
		self.safe_address
	}

	pub fn signing_hash(&self) -> B256 {
		self.signing_hash
	}

	pub fn typed_data(&self) -> &TypedDataJson<CreateProxyMessageJson> {
		&self.typed_data
	}

	pub fn into_submit_request(self, signature: Signature) -> SubmitRequest {
		self.submit_base.into_submit_request(&signature.as_bytes())
	}
}

impl SafeExecutionDraft {
	pub fn safe_address(&self) -> Address {
		self.safe_address
	}

	pub fn aggregated_call(&self) -> &Call {
		&self.aggregated_call
	}

	pub fn operation(&self) -> SafeOperation {
		self.operation
	}

	pub fn signing_hash(&self) -> B256 {
		self.signing_hash
	}

	pub fn personal_sign_payload(&self) -> B256 {
		self.signing_hash
	}

	pub fn into_submit_request(self, signature: PackedSafeSignature) -> SubmitRequest {
		self.submit_base.into_submit_request(signature.as_bytes())
	}
}

pub fn derive_address(owner: Address, safe_factory: Address, safe_init_code_hash: B256) -> Address {
	let mut encoded = [0_u8; 32];
	encoded[12..].copy_from_slice(owner.as_slice());
	let salt = keccak256(encoded);

	create2_address(safe_factory, salt, safe_init_code_hash)
}

pub fn build_create_draft(
	context: &SafeCreateContext,
	payment: &SafeCreatePayment,
) -> SafeCreateDraft {
	let safe_address = derive_address(
		context.owner(),
		context.safe_factory(),
		context.safe_init_code_hash(),
	);
	let domain = Eip712Domain {
		name: Some(context.factory_domain_name().as_str().to_string().into()),
		chain_id: Some(U256::from(context.chain_id().raw())),
		verifying_contract: Some(context.safe_factory()),
		..Default::default()
	};
	let create_proxy = CreateProxy {
		paymentToken: payment.payment_token(),
		payment: payment.payment(),
		paymentReceiver: payment.payment_receiver(),
	};
	let signing_hash = create_proxy.eip712_signing_hash(&domain);
	let typed_data = create_proxy_typed_data_json(context, payment);

	SafeCreateDraft {
		safe_address,
		signing_hash,
		typed_data,
		submit_base: DraftSubmitBase {
			kind: SubmitKind::SafeCreate,
			from: context.owner(),
			to: context.safe_factory(),
			safe_address,
			data: Bytes::new(),
			nonce: None,
			signature_params: SignatureParams::safe_create(payment),
			metadata: None,
		},
	}
}

pub fn build_execution_draft(
	context: &SafeExecutionContext,
	calls: NonEmptyCalls,
) -> SafeExecutionDraft {
	let safe_address = derive_address(
		context.owner(),
		context.safe_factory(),
		context.safe_init_code_hash(),
	);
	let (aggregated_call, operation) = aggregate_calls(calls, context.safe_multisend());
	let domain = Eip712Domain {
		chain_id: Some(U256::from(context.chain_id().raw())),
		verifying_contract: Some(safe_address),
		..Default::default()
	};
	let safe_tx = SafeTx {
		to: aggregated_call.to(),
		value: aggregated_call.value(),
		data: aggregated_call.data().clone(),
		operation: operation.as_u8(),
		safeTxGas: context.gas_params().safe_txn_gas(),
		baseGas: context.gas_params().base_gas(),
		gasPrice: context.gas_params().gas_price(),
		gasToken: context.gas_params().gas_token(),
		refundReceiver: context.gas_params().refund_receiver(),
		nonce: context.nonce().raw(),
	};
	let signing_hash = safe_tx.eip712_signing_hash(&domain);

	SafeExecutionDraft {
		safe_address,
		aggregated_call,
		operation,
		signing_hash,
		submit_base: DraftSubmitBase {
			kind: SubmitKind::Safe,
			from: context.owner(),
			to: safe_tx.to,
			safe_address,
			data: safe_tx.data,
			nonce: Some(context.nonce()),
			signature_params: SignatureParams::safe(operation, context.gas_params()),
			metadata: context.metadata().cloned(),
		},
	}
}

impl DraftSubmitBase {
	fn into_submit_request(self, signature_bytes: &[u8]) -> SubmitRequest {
		SubmitRequest {
			kind: self.kind,
			from: address_string(self.from),
			to: address_string(self.to),
			proxy_wallet: Some(address_string(self.safe_address)),
			data: hex_string(self.data.as_ref()),
			nonce: self.nonce.map(|nonce| nonce.raw().to_string()),
			signature: hex_string(signature_bytes),
			signature_params: self.signature_params,
			metadata: self.metadata.map(|metadata| metadata.as_str().to_string()),
		}
	}
}

fn aggregate_calls(calls: NonEmptyCalls, safe_multisend: Address) -> (Call, SafeOperation) {
	if calls.len().get() == 1 {
		return (
			calls.into_vec().into_iter().next().expect("non-empty"),
			SafeOperation::Call,
		);
	}

	let encoded = encode_multisend_payload(calls.as_slice());
	let data = Bytes::from(IMultiSend::multiSendCall { transactions: encoded.into() }.abi_encode());
	let call = Call::builder().to(safe_multisend).data(data).build();

	(call, SafeOperation::DelegateCall)
}

fn encode_multisend_payload(calls: &[Call]) -> Vec<u8> {
	let mut encoded = Vec::new();

	for call in calls {
		encoded.push(SafeOperation::Call.as_u8());
		encoded.extend_from_slice(call.to().as_slice());
		encoded.extend_from_slice(&call.value().to_be_bytes::<32>());
		encoded.extend_from_slice(&U256::from(call.data().len()).to_be_bytes::<32>());
		encoded.extend_from_slice(call.data().as_ref());
	}

	encoded
}

fn create_proxy_typed_data_json(
	context: &SafeCreateContext,
	payment: &SafeCreatePayment,
) -> TypedDataJson<CreateProxyMessageJson> {
	let mut types = BTreeMap::new();
	types.insert(
		SafeCreateDraft::DOMAIN_TYPE.to_string(),
		eip712_domain_fields(),
	);
	types.insert(
		SafeCreateDraft::PRIMARY_TYPE.to_string(),
		create_proxy_fields(),
	);

	TypedDataJson {
		types,
		primary_type: SafeCreateDraft::PRIMARY_TYPE.to_string(),
		domain: TypedDataDomainJson {
			name: Some(context.factory_domain_name().as_str().to_string()),
			version: None,
			chain_id: Some(context.chain_id().raw()),
			verifying_contract: Some(address_string(context.safe_factory())),
			salt: None,
		},
		message: CreateProxyMessageJson {
			payment_token: address_string(payment.payment_token()),
			payment: payment.payment().to_string(),
			payment_receiver: address_string(payment.payment_receiver()),
		},
	}
}

fn eip712_domain_fields() -> Vec<TypedDataFieldJson> {
	vec![
		TypedDataFieldJson { name: FIELD_NAME_NAME, type_name: SOL_TYPE_STRING },
		TypedDataFieldJson { name: FIELD_NAME_CHAIN_ID, type_name: SOL_TYPE_UINT256 },
		TypedDataFieldJson { name: FIELD_NAME_VERIFYING_CONTRACT, type_name: SOL_TYPE_ADDRESS },
	]
}

fn create_proxy_fields() -> Vec<TypedDataFieldJson> {
	vec![
		TypedDataFieldJson { name: FIELD_NAME_PAYMENT_TOKEN, type_name: SOL_TYPE_ADDRESS },
		TypedDataFieldJson { name: FIELD_NAME_PAYMENT, type_name: SOL_TYPE_UINT256 },
		TypedDataFieldJson { name: FIELD_NAME_PAYMENT_RECEIVER, type_name: SOL_TYPE_ADDRESS },
	]
}

fn create2_address(deployer: Address, salt: B256, init_code_hash: B256) -> Address {
	let mut payload = [0_u8; 85];
	payload[0] = 0xff;
	payload[1..21].copy_from_slice(deployer.as_slice());
	payload[21..53].copy_from_slice(salt.as_slice());
	payload[53..85].copy_from_slice(init_code_hash.as_slice());
	let hash = keccak256(payload);

	Address::from_slice(&hash[12..])
}

fn validate_packed_signature_bytes(bytes: &[u8; 65]) -> Result<(), PolyrelError> {
	if !(PackedSafeSignature::V_MIN..=PackedSafeSignature::V_MAX).contains(&bytes[64]) {
		return Err(PolyrelError::invalid_signature(
			"packed safe signature must use v=31 or v=32",
		));
	}

	Ok(())
}

fn address_string(address: Address) -> String {
	format!("{address:#x}")
}

fn hex_string(bytes: &[u8]) -> String {
	format!("0x{}", alloy_primitives::hex::encode(bytes))
}

#[cfg(test)]
mod tests {
	use alloc::vec;

	use alloy_primitives::{Signature, U256, address, b256};

	use super::*;

	const TEST_OWNER: Address = address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5");
	const TEST_SAFE_FACTORY: Address = address!("aacfeea03eb1561c4e67d661e40682bd20e3541b");
	const TEST_SAFE_MULTISEND: Address = address!("a238cbeb142c10ef7ad8442c6d1f9e89e07e7761");
	const TEST_SAFE_ADDRESS: Address = address!("6d8c4e9adf5748af82dabe2c6225207770d6b4fa");
	const TEST_SINGLE_CALL_TARGET: Address = address!("c011a7e12a19f7b1f670d46f03b03f3342e82dfb");
	const TEST_SECOND_CALL_TARGET: Address = address!("2791bca1f2de4661ed88a30c99a7a9449aa84174");
	const TEST_SAFE_INIT_CODE_HASH: B256 =
		b256!("2bce2127ff07fb632d16c8347c4ebf501f4841168bed00d9e6ef715ddb6fcecf");
	const TEST_FACTORY_DOMAIN_NAME: &str = "Polymarket Contract Proxy Factory";
	const TEST_METADATA: &str = "approve";
	const TEST_CREATE_TYPED_DATA_JSON: &str = r#"{"types":{"CreateProxy":[{"name":"paymentToken","type":"address"},{"name":"payment","type":"uint256"},{"name":"paymentReceiver","type":"address"}],"EIP712Domain":[{"name":"name","type":"string"},{"name":"chainId","type":"uint256"},{"name":"verifyingContract","type":"address"}]},"primaryType":"CreateProxy","domain":{"name":"Polymarket Contract Proxy Factory","chainId":137,"verifyingContract":"0xaacfeea03eb1561c4e67d661e40682bd20e3541b"},"message":{"paymentToken":"0x0000000000000000000000000000000000000000","payment":"0","paymentReceiver":"0x0000000000000000000000000000000000000000"}}"#;
	const TEST_CREATE_HASH: B256 =
		b256!("563ac315294c5be01ab1f3b04a5abdfa39e8317a9d90679d4e63caf760b126a4");
	const TEST_EXECUTION_HASH: B256 =
		b256!("8835f5f740c39b2c57b5fa5f5f67a3c3a4cc5e68cb38bb392f4e239d4b08c044");
	const TEST_MULTISEND_CALLDATA: &str = "8d80ff0a000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000b000c011a7e12a19f7b1f670d46f03b03f3342e82dfb00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004deadbeef002791bca1f2de4661ed88a30c99a7a9449aa8417400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002cafe00000000000000000000000000000000";
	const TEST_PACKED_SIGNATURE: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa1f";
	const TEST_CREATE_REQUEST_JSON: &str = r#"{"type":"SAFE-CREATE","from":"0x6e0c80c90ea6c15917308f820eac91ce2724b5b5","to":"0xaacfeea03eb1561c4e67d661e40682bd20e3541b","proxyWallet":"0x6d8c4e9adf5748af82dabe2c6225207770d6b4fa","data":"0x","signature":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb1b","signatureParams":{"paymentToken":"0x0000000000000000000000000000000000000000","payment":"0","paymentReceiver":"0x0000000000000000000000000000000000000000"}}"#;

	fn create_context() -> SafeCreateContext {
		SafeCreateContext::builder()
			.owner(TEST_OWNER)
			.chain_id(ChainId::new(137.try_into().unwrap()))
			.safe_factory(TEST_SAFE_FACTORY)
			.safe_init_code_hash(TEST_SAFE_INIT_CODE_HASH)
			.factory_domain_name(FactoryDomainName::new(TEST_FACTORY_DOMAIN_NAME.into()).unwrap())
			.build()
	}

	fn create_payment() -> SafeCreatePayment {
		SafeCreatePayment::builder()
			.payment_token(Address::ZERO)
			.payment(U256::ZERO)
			.payment_receiver(Address::ZERO)
			.build()
	}

	fn execution_context() -> SafeExecutionContext {
		SafeExecutionContext::builder()
			.owner(TEST_OWNER)
			.chain_id(ChainId::new(137.try_into().unwrap()))
			.safe_factory(TEST_SAFE_FACTORY)
			.safe_init_code_hash(TEST_SAFE_INIT_CODE_HASH)
			.safe_multisend(TEST_SAFE_MULTISEND)
			.nonce(SafeNonce::new(U256::from(60_u64)))
			.gas_params(
				SafeGasParams::builder()
					.safe_txn_gas(U256::ZERO)
					.base_gas(U256::ZERO)
					.gas_price(U256::ZERO)
					.gas_token(Address::ZERO)
					.refund_receiver(Address::ZERO)
					.build(),
			)
			.metadata(Metadata::new(TEST_METADATA.into()))
			.build()
	}

	fn approval_call() -> Call {
		Call::builder()
			.to(TEST_SINGLE_CALL_TARGET)
			.data(Bytes::from_static(&[0xde, 0xad, 0xbe, 0xef]))
			.build()
	}

	fn wallet_signature(fill: u8, v: u8) -> Signature {
		let mut bytes = [fill; 65];
		bytes[64] = v;

		Signature::from_raw_array(&bytes).unwrap()
	}

	#[test]
	fn derive_address_matches_expected_fixture() {
		// Arrange
		let context = create_context();

		// Act
		let address = derive_address(
			context.owner(),
			context.safe_factory(),
			context.safe_init_code_hash(),
		);

		// Assert
		assert_eq!(address, TEST_SAFE_ADDRESS);
	}

	#[test]
	fn create_draft_serializes_expected_typed_data() {
		// Arrange
		let draft = build_create_draft(&create_context(), &create_payment());

		// Act
		let json = serde_json::to_string(draft.typed_data()).unwrap();

		// Assert
		assert_eq!(json, TEST_CREATE_TYPED_DATA_JSON);
	}

	#[test]
	fn create_draft_hash_matches_expected_fixture() {
		// Arrange
		let draft = build_create_draft(&create_context(), &create_payment());

		// Act
		let hash = draft.signing_hash();

		// Assert
		assert_eq!(hash, TEST_CREATE_HASH);
	}

	#[test]
	fn execution_draft_hash_matches_expected_fixture() {
		// Arrange
		let calls = NonEmptyCalls::from_one(approval_call());

		// Act
		let draft = build_execution_draft(&execution_context(), calls);

		// Assert
		assert_eq!(draft.operation(), SafeOperation::Call);
		assert_eq!(draft.signing_hash(), TEST_EXECUTION_HASH);
	}

	#[test]
	fn execution_multisend_aggregates_calls() {
		// Arrange
		let first = approval_call();
		let second = Call::builder()
			.to(TEST_SECOND_CALL_TARGET)
			.data(Bytes::from_static(&[0xca, 0xfe]))
			.build();
		let calls = NonEmptyCalls::new(vec![first, second]).unwrap();

		// Act
		let draft = build_execution_draft(&execution_context(), calls);

		// Assert
		assert_eq!(draft.operation(), SafeOperation::DelegateCall);
		assert_eq!(
			alloy_primitives::hex::encode(draft.aggregated_call().data()),
			TEST_MULTISEND_CALLDATA
		);
	}

	#[test]
	fn execution_submit_request_packs_wallet_signature() {
		// Arrange
		let calls = NonEmptyCalls::from_one(approval_call());
		let draft = build_execution_draft(&execution_context(), calls);
		let signature = PackedSafeSignature::from_wallet_signature(wallet_signature(0xaa, 27));

		// Act
		let request = draft.into_submit_request(signature);

		// Assert
		assert_eq!(request.kind, SubmitKind::Safe);
		assert_eq!(request.signature, TEST_PACKED_SIGNATURE);
		assert_eq!(request.nonce.as_deref(), Some("60"));
	}

	#[test]
	fn create_submit_request_serializes_expected_fixture() {
		// Arrange
		let request = build_create_draft(&create_context(), &create_payment())
			.into_submit_request(wallet_signature(0xbb, 27));

		// Act
		let json = serde_json::to_string(&request).unwrap();

		// Assert
		assert_eq!(json, TEST_CREATE_REQUEST_JSON);
	}

	#[test]
	fn packed_safe_signature_rejects_non_safe_v_values() {
		// Arrange
		let mut bytes = [0x11; 65];
		bytes[64] = 27;

		// Act
		let result = PackedSafeSignature::new(bytes);

		// Assert
		assert!(matches!(result, Err(PolyrelError::InvalidSignature(_))));
	}
}
