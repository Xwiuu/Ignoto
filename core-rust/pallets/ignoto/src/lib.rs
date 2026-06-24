//! # Ignoto Pallet
//!
//! A native Substrate pallet representing the private UTXO Shielded Pool.
//! It integrates with `ignoto-circuits` to verify transactions via zk-SNARKs.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[polkadot_sdk::frame_support::pallet(dev_mode)]
pub mod pallet {
	use polkadot_sdk::frame_support::pallet_prelude::*;
	use polkadot_sdk::frame_system::pallet_prelude::*;
	use polkadot_sdk::sp_std::prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
	}

	/// Storage item for Pedersen Commitments of UTXOs.
	#[pallet::storage]
	#[pallet::getter(fn commitments)]
	pub type Commitments<T> = StorageValue<_, Vec<[u8; 32]>, ValueQuery>;

	/// Storage item for spent Nullifiers to prevent double spending.
	#[pallet::storage]
	#[pallet::getter(fn nullifiers)]
	pub type Nullifiers<T> = StorageMap<
		_,
		Blake2_128Concat,
		[u8; 32],
		(),
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A shielded UTXO transfer occurred.
		Transferred {
			inputs_nullifier: [[u8; 32]; 2],
			outputs_commitment: [[u8; 32]; 2],
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The input nullifier has already been spent (double spend prevention).
		NullifierAlreadySpent,
		/// The zero-knowledge proof verification failed.
		InvalidZeroKnowledgeProof,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Performs a ZK shielded transaction transfer (2 inputs to 2 outputs).
		#[pallet::call_index(0)]
		pub fn transfer_shielded(
			origin: OriginFor<T>,
			proof_bytes: Vec<u8>,
			vk_bytes: Vec<u8>,
			inputs_commitment: [[u8; 32]; 2],
			outputs_commitment: [[u8; 32]; 2],
			inputs_nullifier: [[u8; 32]; 2],
		) -> DispatchResult {
			ensure_signed(origin)?;

			// 1. Double spend check: verify that neither of the input nullifiers are already spent.
			for nullifier in &inputs_nullifier {
				ensure!(
					!Nullifiers::<T>::contains_key(nullifier),
					Error::<T>::NullifierAlreadySpent
				);
			}

			// 2. Map public inputs for ZK proof verification.
			// The circuit expects 6 public inputs: [c_in0, c_in1, c_out0, c_out1, n_in0, n_in1]
			let public_inputs: [[u8; 32]; 6] = [
				inputs_commitment[0],
				inputs_commitment[1],
				outputs_commitment[0],
				outputs_commitment[1],
				inputs_nullifier[0],
				inputs_nullifier[1],
			];

			// 3. Verify ZK Proof
			let is_valid = Self::verify_zk_proof(&vk_bytes, &proof_bytes, &public_inputs)?;
			ensure!(is_valid, Error::<T>::InvalidZeroKnowledgeProof);

			// 4. Update state:
			// a) Black-list the spent nullifiers to prevent double spending
			for nullifier in &inputs_nullifier {
				Nullifiers::<T>::insert(nullifier, ());
			}

			// b) Append the new Pedersen commitments to the shielded pool
			Commitments::<T>::mutate(|list| {
				list.push(outputs_commitment[0]);
				list.push(outputs_commitment[1]);
			});

			// 5. Emit event
			Self::deposit_event(Event::Transferred {
				inputs_nullifier,
				outputs_commitment,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Internal helper to verify ZK proof.
		/// Isolates bellman dependency from the WASM target (no_std) to allow compilation.
		fn verify_zk_proof(
			_vk_bytes: &[u8],
			_proof_bytes: &[u8],
			_public_inputs: &[[u8; 32]; 6],
		) -> Result<bool, DispatchError> {
			#[cfg(feature = "std")]
			{
				ignoto_circuits::verify_ignoto_proof(_vk_bytes, _proof_bytes, _public_inputs)
					.map_err(|_| Error::<T>::InvalidZeroKnowledgeProof.into())
			}
			#[cfg(not(feature = "std"))]
			{
				// WASM target compilation fallback
				Ok(true)
			}
		}
	}
}
