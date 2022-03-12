#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod pool;

#[frame_support::pallet]
pub mod pallet {
	use crate::pool::{AuroraZone, PackService, PackServiceProvider, Player, Service};
	use frame_support::{
		dispatch::{DispatchResult, Vec},
		pallet_prelude::*,
		traits::{
			tokens::{ExistenceRequirement, WithdrawReasons},
			Currency,
		},
	};
	use frame_system::pallet_prelude::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types it depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type MaxNewPlayer: Get<u32>;

		#[pallet::constant]
		type MaxIngamePlayer: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		PlayerNotFound,
		PlayerAlreadyJoin,

		PlayerCountOverflow,
		ExceedMaxPlayer,
		ExceedMaxNewPlayer,
		CanNotClearNewPlayers,
		ExceedMaxIngamePlayer,
		CanNotCalculateRefundFee,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PlayerJoinPool(T::AccountId),
		PlayerLeavePool(T::AccountId),
	}

	/*
		1. Charge player in the IngamePlayers
			1.1 Kick player when they can't pay
		2. Move all players from NewPlayer to IngamePlayers
	*/
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(block_number: BlockNumberFor<T>) {
			let mark_block = Self::mark_block();

			if let Some(block) = Self::block_to_u64(block_number) {
				if block % mark_block == 0 {
					let _ = Self::charge_ingame();
					let _ = Self::move_newplayer_to_ingame();
				}
			}
		}
	}

	//** Storage **//
	#[pallet::storage]
	#[pallet::getter(fn max_player)]
	pub type MaxPlayer<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mark_block)]
	pub type MarkBlock<T: Config> = StorageValue<_, u64, ValueQuery>;

	// Store all players join the pool
	#[pallet::storage]
	pub(super) type Players<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Player<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn player_count)]
	pub(super) type PlayerCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub(super) type NewPlayers<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::MaxNewPlayer>, ValueQuery>;

	#[pallet::storage]
	pub type IngamePlayers<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::MaxIngamePlayer>, ValueQuery>;

	#[pallet::type_value]
	pub(super) fn DefaultService<T: Config>() -> Service<BalanceOf<T>> {
		Service { tx_limit: 4, discount: 60, service: 1000_000_000u128.try_into().ok().unwrap() }
	}
	#[pallet::storage]
	#[pallet::getter(fn services)]
	pub(super) type Services<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PackService,
		Service<BalanceOf<T>>,
		ValueQuery,
		DefaultService<T>,
	>;

	//** Genesis Conguration **//
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub max_player: u32,
		pub mark_block: u64,
		pub services: [(PackService, u8, u8, BalanceOf<T>); 3],
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			const BASE_FEE: u64 = 1000_000;
			let convert_default_fee = |fee: u64| -> BalanceOf<T> { fee.try_into().ok().unwrap() };
			Self {
				max_player: 1000,
				mark_block: 30,
				services: [
					(PackService::Basic, 4, 60, convert_default_fee(BASE_FEE)),
					(PackService::Medium, 8, 70, convert_default_fee(BASE_FEE * 2)),
					(PackService::Max, u8::MAX, 80, convert_default_fee(BASE_FEE * 3)),
				],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<MaxPlayer<T>>::put(self.max_player);
			<MarkBlock<T>>::put(self.mark_block);

			for service in self.services.iter() {
				let new_service =
					Service { tx_limit: service.1, discount: service.2, service: service.3 };
				<Services<T>>::insert(service.0, new_service);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/*
			* player join the pool
			* 1. Add new player to NewPlayer
			* 2. charge double service fee when they join
			*/
		#[pallet::weight(100)]
		pub fn join(origin: OriginFor<T>, pack: PackService) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::join_pool(sender.clone(), pack)?;
			let pack_service = Services::<T>::get(pack);
			let double_fee = pack_service.service * 2u32.into();
			Self::change_fee(&sender, double_fee)?;
			Self::deposit_event(Event::PlayerJoinPool(sender));

			Ok(())
		}

		/*
			* player leave the pool
			* 1. remove player from storages
			* 2. refund appropriate amount (the maximum amount they receive is 'service_fee'/2)
			*/
		#[pallet::weight(100)]
		pub fn leave(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::leave_pool(&sender)?;
			Self::deposit_event(Event::PlayerLeavePool(sender));
			Ok(())
		}

		/*
			* Set new MaxPlayer for the pool, MaxPlayer must be <= MaxNewPlayer and <= MaxIngamePlayer
			*/
		#[pallet::weight(0)]
		pub fn set_max_player(origin: OriginFor<T>, max_player: u32) -> DispatchResult {
			ensure_root(origin)?;
			// make sure new max_player not exceed the capacity of NewPlayers and IngamePlayers
			ensure!(max_player <= T::MaxNewPlayer::get(), <Error<T>>::ExceedMaxNewPlayer);
			ensure!(max_player <= T::MaxIngamePlayer::get(), <Error<T>>::ExceedMaxIngamePlayer);
			<MaxPlayer<T>>::put(max_player);
			Ok(())
		}

		/*
			* Set new pack service
			*/
		#[pallet::weight(0)]
		pub fn set_pack_service(
			origin: OriginFor<T>,
			pack: PackService,
			tx_limit: u8,
			discount: u8,
			service: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let pack_service = Service { tx_limit, discount, service };
			Services::<T>::insert(pack, pack_service);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn join_pool(sender: T::AccountId, pack: PackService) -> Result<(), Error<T>> {
			// make sure player not re-join
			ensure!(Players::<T>::get(sender.clone()) == None, <Error<T>>::PlayerAlreadyJoin);
			// make sure not exceed max players
			let new_player_count =
				Self::player_count().checked_add(1).ok_or(<Error<T>>::PlayerCountOverflow)?;
			ensure!(new_player_count <= Self::max_player(), <Error<T>>::ExceedMaxPlayer);
			// make sure not exceed max new players
			<NewPlayers<T>>::try_mutate(|newplayers| newplayers.try_push(sender.clone()))
				.map_err(|_| <Error<T>>::ExceedMaxNewPlayer)?;
			let block_number = Self::get_block_number();
			let player = Player::<T::AccountId> {
				address: sender.clone(),
				join_block: block_number,
				service: pack,
			};
			<Players<T>>::insert(sender, player);
			<PlayerCount<T>>::put(new_player_count);
			Ok(())
		}

		pub fn change_fee(sender: &T::AccountId, fee: BalanceOf<T>) -> DispatchResult {
			let withdraw = T::Currency::withdraw(
				&sender,
				fee,
				WithdrawReasons::RESERVE,
				ExistenceRequirement::KeepAlive,
			);

			match withdraw {
				Ok(_) => Ok(()),
				Err(err) => Err(err),
			}
		}

		/*
			1. Calculate fee to refund
			2. Remove sender from Players and NewPlayers/IngamePlayers
		*/
		fn leave_pool(sender: &T::AccountId) -> Result<(), Error<T>> {
			if let Some(player) = Players::<T>::get(sender) {
				let join_block = player.join_block;
				let block_number = Self::get_block_number();
				let range_block = block_number - join_block;
				let mut refund_fee: BalanceOf<T>;
				let pack = Services::<T>::get(player.service);
				refund_fee = pack.service;

				if range_block < Self::mark_block() {
					<NewPlayers<T>>::try_mutate(|players| {
						if let Some(ind) = players.iter().position(|id| id == sender) {
							players.swap_remove(ind);
						}
						Ok(())
					})
					.map_err(|_: Error<T>| <Error<T>>::PlayerNotFound)?;
				} else {
					<IngamePlayers<T>>::try_mutate(|players| {
						if let Some(ind) = players.iter().position(|id| id == sender) {
							players.swap_remove(ind);
						}
						Ok(())
					})
					.map_err(|_: Error<T>| <Error<T>>::PlayerNotFound)?;

					refund_fee = Self::calculate_ingame_refund_amount(
						join_block,
						block_number,
						player.service,
					)?;
				}
				<Players<T>>::remove(sender);
				let _ = T::Currency::deposit_into_existing(sender, refund_fee);
			} else {
				return Err(<Error<T>>::PlayerNotFound);
			}
			Ok(())
		}

		pub fn calculate_ingame_refund_amount(
			join_block: u64,
			block_number: u64,
			service: PackService,
		) -> Result<BalanceOf<T>, Error<T>> {
			let range_block = block_number - join_block;
			let extra = range_block % Self::mark_block();
			let service = Services::<T>::get(service);
			if let Some(fee) = Self::balance_to_u64(service.service) {
				let actual_fee = fee * (Self::mark_block() - extra) / Self::mark_block();
				if let Some(result) = Self::u64_to_balance(actual_fee) {
					return Ok(result);
				}
			}
			return Err(<Error<T>>::CanNotCalculateRefundFee);
		}

		/*
		 */
		fn kick_ingame_player(player: &T::AccountId) -> Result<(), Error<T>> {
			<Players<T>>::remove(player);
			<IngamePlayers<T>>::try_mutate(|players| {
				if let Some(ind) = players.iter().position(|id| id == player) {
					players.swap_remove(ind);
				}
				Ok(())
			})
			.map_err(|_: Error<T>| <Error<T>>::PlayerNotFound)?;
			Ok(())
		}

		fn charge_ingame() -> Result<(), Error<T>> {
			let ingame_players: Vec<T::AccountId> = IngamePlayers::<T>::get().into_inner();
			for player in ingame_players {
				if let Some(ingame_player) = Players::<T>::get(&player) {
					let pack = Services::<T>::get(ingame_player.service);
					match Self::change_fee(&player, pack.service) {
						Ok(_) => {},
						Err(_) => {
							let _ = Self::kick_ingame_player(&player);
						},
					}
				}
			}
			Ok(())
		}

		fn move_newplayer_to_ingame() -> Result<(), Error<T>> {
			let new_players: Vec<T::AccountId> = NewPlayers::<T>::get().into_inner();
			for new_player in new_players {
				<IngamePlayers<T>>::try_append(new_player)
					.map_err(|_| <Error<T>>::ExceedMaxNewPlayer)?;
			}
			<NewPlayers<T>>::kill();
			Ok(())
		}

		pub fn block_to_u64(input: T::BlockNumber) -> Option<u64> {
			TryInto::<u64>::try_into(input).ok()
		}

		pub fn balance_to_u64(input: BalanceOf<T>) -> Option<u64> {
			TryInto::<u64>::try_into(input).ok()
		}

		pub fn u64_to_balance(input: u64) -> Option<BalanceOf<T>> {
			input.try_into().ok()
		}

		pub fn u64_to_block(input: u64) -> Option<T::BlockNumber> {
			input.try_into().ok()
		}

		/*
			Return current block number otherwise return 0
		*/
		pub fn get_block_number() -> u64 {
			let block_number = <frame_system::Pallet<T>>::block_number();
			if let Some(block) = Self::block_to_u64(block_number) {
				return block;
			}
			return 0u64;
		}
	}

	impl<T: Config> AuroraZone<T::AccountId> for Pallet<T> {
		fn is_in_aurora_zone(player: &T::AccountId) -> Option<Player<T::AccountId>> {
			Players::<T>::get(player)
		}
	}

	impl<T: Config> PackServiceProvider<BalanceOf<T>> for Pallet<T> {
		fn get_service(service: PackService) -> Option<Service<BalanceOf<T>>> {
			Some(Services::<T>::get(service))
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
