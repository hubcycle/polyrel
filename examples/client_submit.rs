#[cfg(feature = "client")]
fn main() {
	use alloy_primitives::{Address, B256, Signature, U256, address, b256};
	use polyrel::{
		client::{RelayerApiKeyAuth, RelayerBaseUrl, RelayerClient},
		safe::{
			ChainId, FactoryDomainName, SafeCreateContext, SafeCreatePayment, build_create_draft,
		},
	};
	use secrecy::SecretString;
	use std::borrow::Cow;

	const BASE_URL: &str = "https://relayer-v2.polymarket.com";
	const API_KEY: &str = "replace-me";
	const OWNER: Address = address!("6e0c80c90ea6c15917308f820eac91ce2724b5b5");
	const SAFE_FACTORY: Address = address!("aacfeea03eb1561c4e67d661e40682bd20e3541b");
	const SAFE_INIT_CODE_HASH: B256 =
		b256!("2bce2127ff07fb632d16c8347c4ebf501f4841168bed00d9e6ef715ddb6fcecf");
	const FACTORY_DOMAIN_NAME: &str = "Polymarket Contract Proxy Factory";
	let base_url = RelayerBaseUrl::parse(Cow::Borrowed(BASE_URL)).unwrap();
	let client = RelayerClient::new(base_url)
		.authenticate(RelayerApiKeyAuth::new(SecretString::from(API_KEY), OWNER));
	let draft = build_create_draft(
		&SafeCreateContext::builder()
			.owner(OWNER)
			.chain_id(ChainId::new(137.try_into().unwrap()))
			.safe_factory(SAFE_FACTORY)
			.safe_init_code_hash(SAFE_INIT_CODE_HASH)
			.factory_domain_name(FactoryDomainName::new(FACTORY_DOMAIN_NAME.into()).unwrap())
			.build(),
		&SafeCreatePayment::builder()
			.payment_token(Address::ZERO)
			.payment(U256::ZERO)
			.payment_receiver(Address::ZERO)
			.build(),
	);
	let signature = {
		let mut bytes = [0x11; 65];
		bytes[64] = 27;
		Signature::from_raw_array(&bytes).unwrap()
	};
	let request = draft.into_submit_request(signature);

	let _unused = client;
	println!("Prepared request kind {:?}", request.kind);
}

#[cfg(not(feature = "client"))]
fn main() {
	const CLIENT_FEATURE_HINT: &str = "Run this example with `--features client`.";

	eprintln!("{CLIENT_FEATURE_HINT}");
}
