use crate::*;
use frame_support::{pallet_prelude::*, traits::BalanceStatus};
use gafi_support::game::{Bundle, Wishlist};

impl<T: Config<I>, I: 'static>
	Wishlist<T::AccountId, T::CollectionId, T::ItemId, T::TradeId, BalanceOf<T, I>, BlockNumber<T>> for Pallet<T, I>
{
	fn do_set_wishlist(
		trade: &T::TradeId,
		who: &T::AccountId,
		wishlist: Bundle<T::CollectionId, T::ItemId>,
		price: BalanceOf<T, I>,
		start_block: Option<T::BlockNumber>,
		end_block: Option<T::BlockNumber>,
	) -> DispatchResult {
		// ensure available trade
		ensure!(
			!BundleOf::<T, I>::contains_key(trade),
			Error::<T, I>::TradeIdInUse
		);

		<T as Config<I>>::Currency::reserve(&who, T::BundleDeposit::get())?;
		<T as Config<I>>::Currency::reserve(&who, price)?;

		<BundleOf<T, I>>::try_mutate(trade, |package_vec| -> DispatchResult {
			package_vec
				.try_append(wishlist.clone().into_mut())
				.map_err(|_| Error::<T, I>::ExceedMaxBundle)?;
			Ok(())
		})?;

		TradeConfigOf::<T, I>::insert(
			trade,
			TradeConfig {
				trade: TradeType::Wishlist,
				owner: who.clone(),
				maybe_price: Some(price),
				maybe_required: None,
				start_block,
				end_block,
			},
		);

		Self::deposit_event(Event::<T, I>::WishlistSet {
			trade: *trade,
			who: who.clone(),
			wishlist,
			price,
		});

		Ok(())
	}

	fn do_claim_wishlist(
		trade: &T::TradeId,
		who: &T::AccountId,
		ask_price: BalanceOf<T, I>,
	) -> DispatchResult {
		if let Some(config) = TradeConfigOf::<T, I>::get(trade) {

			let block_number = <frame_system::Pallet<T>>::block_number();
			if let Some(start_block) = config.start_block {
				ensure!(block_number >= start_block, Error::<T, I>::TradeNotStarted);
			}
			if let Some(end_block) = config.end_block {
				ensure!(block_number <= end_block, Error::<T, I>::TradeEnded);
			}

			ensure!(
				config.trade == TradeType::Wishlist,
				Error::<T, I>::NotWishlist
			);

			let bundle = BundleOf::<T, I>::get(trade);
			// ensure item can be transfer
			for pack in bundle.clone() {
				ensure!(
					T::Nfts::can_transfer(&pack.collection, &pack.item),
					Error::<T, I>::ItemLocked
				);
			}

			let price = config.maybe_price.unwrap_or_default();

			// check price
			ensure!(ask_price <= price, Error::<T, I>::AskTooHigh);

			// make deposit
			<T as pallet::Config<I>>::Currency::repatriate_reserved(
				&config.owner,
				&who,
				price,
				BalanceStatus::Free,
			)?;

			// transfer items
			for package in bundle.clone() {
				Self::transfer_item(
					&who,
					&package.collection,
					&package.item,
					&config.owner,
					package.amount,
				)?;
			}

			// end trade
			<T as pallet::Config<I>>::Currency::unreserve(&config.owner, T::BundleDeposit::get());
			BundleOf::<T, I>::remove(trade);
			TradeConfigOf::<T, I>::remove(trade);

			Self::deposit_event(Event::<T, I>::WishlistFilled {
				trade: *trade,
				who: who.clone(),
				ask_price,
			});
			return Ok(())
		}
		return Err(Error::<T, I>::UnknownTrade.into())
	}

	fn do_cancel_wishlist(trade: &T::TradeId, who: &T::AccountId) -> DispatchResult {
		if let Some(config) = TradeConfigOf::<T, I>::get(trade) {
			ensure!(
				config.trade == TradeType::Wishlist,
				Error::<T, I>::NotWishlist
			);

			// ensure owner
			ensure!(who.eq(&config.owner), Error::<T, I>::NoPermission);

			// end trade
			<T as pallet::Config<I>>::Currency::unreserve(&config.owner, T::BundleDeposit::get());
			BundleOf::<T, I>::remove(trade);
			TradeConfigOf::<T, I>::remove(trade);

			Self::deposit_event(Event::<T, I>::TradeCanceled {
				trade: *trade,
				who: who.clone(),
			});
			return Ok(())
		}
		Err(Error::<T, I>::UnknownTrade.into())
	}
}
