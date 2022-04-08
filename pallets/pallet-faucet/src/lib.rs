#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, ExistenceRequirement};
pub use pallet::*;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, BoundedVec};
	use frame_system::pallet_prelude::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type MaxGenesisAccount: Get<u32>;

		#[pallet::constant]
		type FaucetBalance: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MinFaucetBalance: Get<BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type GenesisAccounts<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::MaxGenesisAccount>, ValueQuery>;

	//** Genesis Conguration **//
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub genesis_accounts: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { genesis_accounts: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for i in 0..self.genesis_accounts.len() {
				<GenesisAccounts<T>>::try_append(self.genesis_accounts[i].clone())
					.map_or((), |_| {});
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		DontBeGreedy,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((
			0,
			DispatchClass::Normal,
			Pays::No
		))]
		pub fn faucet(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let genesis_accounts = GenesisAccounts::<T>::get();

			ensure!(
				T::Currency::free_balance(&sender) < T::MinFaucetBalance::get(),
				<Error<T>>::DontBeGreedy
			);

			for account in genesis_accounts {
				match T::Currency::transfer(
					&account,
					&sender,
					T::FaucetBalance::get(),
					ExistenceRequirement::KeepAlive,
				) {
					Ok(_) => return Ok(()),
					Err(_) => continue,
				}
			}
			Err(DispatchError::Other("Out of Faucet"))
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> GenesisConfig<T> {
	pub fn build_storage(&self) -> Result<sp_runtime::Storage, String> {
		<Self as frame_support::pallet_prelude::GenesisBuild<T>>::build_storage(self)
	}

	pub fn assimilate_storage(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
		<Self as frame_support::pallet_prelude::GenesisBuild<T>>::assimilate_storage(self, storage)
	}
}
