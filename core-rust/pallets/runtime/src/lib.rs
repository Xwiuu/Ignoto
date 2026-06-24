//! Ignoto Protocol Runtime.
//!
//! A minimal, high-performance runtime containing only the essential pallets:
//! System, Timestamp, Balances (for IGTO token), and Transaction Payment.

#![cfg_attr(not(feature = "std"), no_std)]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

use alloc::vec::Vec;
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use polkadot_sdk::{
	polkadot_sdk_frame::{
		self as frame,
		deps::sp_genesis_builder,
		runtime::{apis, prelude::*},
	},
	*,
};

/// Provides getters for genesis configuration presets.
pub mod genesis_config_presets {
	use super::*;
	use crate::{
		interface::{Balance, MinimumBalance},
		BalancesConfig, RuntimeGenesisConfig,
	};

	use alloc::{vec, vec::Vec};
	use serde_json::Value;

	/// Returns a development genesis config preset.
	pub fn development_config_genesis() -> Value {
		let endowment = <MinimumBalance as Get<Balance>>::get().max(1) * 1000000; // Development supply
		frame_support::build_struct_json_patch!(RuntimeGenesisConfig {
			balances: BalancesConfig {
				balances: polkadot_sdk::sp_keyring::Sr25519Keyring::iter()
					.map(|a| (a.to_account_id(), endowment))
					.collect::<Vec<_>>(),
			},
		})
	}

	/// Get the set of the available genesis config presets.
	pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
		let patch = match id.as_ref() {
			sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
			_ => return None,
		};
		Some(
			serde_json::to_string(&patch)
				.expect("Serialization to JSON must succeed. qed.")
				.into_bytes(),
		)
	}

	/// List of supported presets.
	pub fn preset_names() -> Vec<PresetId> {
		vec![PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET)]
	}
}

/// The runtime version.
#[runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("ignoto-runtime"),
	impl_name: alloc::borrow::Cow::Borrowed("ignoto-runtime"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// Transaction extensions (signed extensions) setup.
type TxExtension = (
	// Checks that the sender is not the zero address.
	frame_system::CheckNonZeroSender<Runtime>,
	// Checks that the runtime version is correct.
	frame_system::CheckSpecVersion<Runtime>,
	// Checks that the transaction version is correct.
	frame_system::CheckTxVersion<Runtime>,
	// Checks that the genesis hash is correct.
	frame_system::CheckGenesis<Runtime>,
	// Checks that the era is valid.
	frame_system::CheckEra<Runtime>,
	// Checks that the nonce is valid.
	frame_system::CheckNonce<Runtime>,
	// Checks that the weight is valid.
	frame_system::CheckWeight<Runtime>,
	// Ensures that the sender has enough funds to pay for the transaction and deducts the fee.
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	// Reclaim the unused weight from the block using post dispatch information.
	frame_system::WeightReclaim<Runtime>,
);

// Composes the runtime by adding all the used pallets and deriving necessary types.
#[frame_construct_runtime]
mod runtime {
	/// The main runtime type.
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Runtime;

	/// Mandatory system pallet that should always be included in a FRAME runtime.
	#[runtime::pallet_index(0)]
	pub type System = frame_system::Pallet<Runtime>;

	/// Provides a way for consensus systems to set and check the onchain time.
	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp::Pallet<Runtime>;

	/// Provides the ability to keep track of balances (IGTO).
	#[runtime::pallet_index(2)]
	pub type Balances = pallet_balances::Pallet<Runtime>;

	/// Provides the ability to charge for extrinsic execution.
	#[runtime::pallet_index(3)]
	pub type TransactionPayment = pallet_transaction_payment::Pallet<Runtime>;

	/// Private UTXO Shielded Pool Pallet.
	#[runtime::pallet_index(4)]
	pub type Ignoto = pallet_ignoto::Pallet<Runtime>;
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
}

// Implements the types required for the system pallet.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	type Block = Block;
	type Version = Version;
	// Use the account data from the balances pallet
	type AccountData = pallet_balances::AccountData<<Runtime as pallet_balances::Config>::Balance>;
}

// Implements the types required for the balances pallet.
#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
	type AccountStore = System;
}

// Implements the types required for the timestamp pallet.
#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Runtime {}

// Implements the types required for the transaction payment pallet.
#[derive_impl(pallet_transaction_payment::config_preludes::TestDefaultConfig)]
impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
	// Fee config for dev purposes
	type WeightToFee = NoFee<<Self as pallet_balances::Config>::Balance>;
	type LengthToFee = FixedFee<1, <Self as pallet_balances::Config>::Balance>;
}

// Implements the types required for the ignoto pallet.
impl pallet_ignoto::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

type Block = frame::runtime::types_common::BlockOf<Runtime, TxExtension>;
type Header = HeaderFor<Runtime>;

type RuntimeExecutive =
	Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllPalletsWithSystem>;

impl_runtime_apis! {
	impl apis::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as frame::traits::Block>::LazyBlock) {
			RuntimeExecutive::execute_block(block)
		}

		fn initialize_block(header: &Header) -> ExtrinsicInclusionMode {
			RuntimeExecutive::initialize_block(header)
		}
	}
	impl apis::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl apis::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: ExtrinsicFor<Runtime>) -> ApplyExtrinsicResult {
			RuntimeExecutive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> HeaderFor<Runtime> {
			RuntimeExecutive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<ExtrinsicFor<Runtime>> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: <Block as frame::traits::Block>::LazyBlock,
			data: InherentData,
		) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl apis::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: ExtrinsicFor<Runtime>,
			block_hash: <Runtime as frame_system::Config>::Hash,
		) -> TransactionValidity {
			RuntimeExecutive::validate_transaction(source, tx, block_hash)
		}
	}

	impl apis::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &HeaderFor<Runtime>) {
			RuntimeExecutive::offchain_worker(header)
		}
	}

	impl apis::SessionKeys<Block> for Runtime {
		fn generate_session_keys(_seed: Option<Vec<u8>>) -> Vec<u8> {
			Default::default()
		}

		fn decode_session_keys(
			_encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, apis::KeyTypeId)>> {
			Default::default()
		}
	}

	impl apis::AccountNonceApi<Block, interface::AccountId, interface::Nonce> for Runtime {
		fn account_nonce(account: interface::AccountId) -> interface::Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		interface::Balance,
	> for Runtime {
		fn query_info(uxt: ExtrinsicFor<Runtime>, len: u32) -> RuntimeDispatchInfo<interface::Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: ExtrinsicFor<Runtime>, len: u32) -> FeeDetails<interface::Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> interface::Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> interface::Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl apis::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, self::genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<PresetId> {
			self::genesis_config_presets::preset_names()
		}
	}
}

/// Interface types re-exported for node consumption.
pub mod interface {
	use super::Runtime;
	use polkadot_sdk::{polkadot_sdk_frame as frame, *};

	pub type Block = super::Block;
	pub use frame::runtime::types_common::OpaqueBlock;
	pub type AccountId = <Runtime as frame_system::Config>::AccountId;
	pub type Nonce = <Runtime as frame_system::Config>::Nonce;
	pub type Hash = <Runtime as frame_system::Config>::Hash;
	pub type Balance = <Runtime as pallet_balances::Config>::Balance;
	pub type MinimumBalance = <Runtime as pallet_balances::Config>::ExistentialDeposit;
}
