use crate::*;
use frame_support::{pallet_prelude::*, traits::ExistenceRequirement};
use gafi_support::game::{Bundle, TradeType, Wholesale};

impl<T: Config<I>, I: 'static>
	Wholesale<T::AccountId, T::CollectionId, T::ItemId, T::TradeId, BalanceOf<T, I>, BlockNumber<T>> for Pallet<T, I>
{
	fn do_set_bundle(
		trade: &T::TradeId,
		who: &T::AccountId,
		bundle: Bundle<T::CollectionId, T::ItemId>,
		price: BalanceOf<T, I>,
		start_block: Option<T::BlockNumber>,
		end_block: Option<T::BlockNumber>,
	) -> DispatchResult {
		// ensure available trade
		ensure!(
			!BundleOf::<T, I>::contains_key(trade),
			Error::<T, I>::TradeIdInUse,
		);

		<T as Config<I>>::Currency::reserve(&who, T::BundleDeposit::get())?;

		// lock bundle
		for package in bundle.clone() {
			Self::reserved_item(who, &package.collection, &package.item, package.amount)?;
		}

		<BundleOf<T, I>>::try_mutate(trade, |package_vec| -> DispatchResult {
			package_vec
				.try_append(bundle.clone().into_mut())
				.map_err(|_| Error::<T, I>::ExceedMaxBundle)?;
			Ok(())
		})?;

		TradeConfigOf::<T, I>::insert(
			trade,
			TradeConfig {
				trade: TradeType::Bundle,
				owner: who.clone(),
				maybe_price: Some(price),
				maybe_required: None,
				start_block,
				end_block,
			},
		);

		Self::deposit_event(Event::<T, I>::BundleSet {
			trade: *trade,
			who: who.clone(),
			bundle,
			price,
		});

		Ok(())
	}

	fn do_buy_bundle(
		trade: &T::TradeId,
		who: &T::AccountId,
		bid_price: BalanceOf<T, I>,
	) -> DispatchResult {
		if let Some(config) = TradeConfigOf::<T, I>::get(trade) {
			ensure!(config.trade == TradeType::Bundle, Error::<T, I>::NotBundle);

			let block_number = <frame_system::Pallet<T>>::block_number();
			if let Some(start_block) = config.start_block {
				ensure!(block_number >= start_block, Error::<T, I>::TradeNotStarted);
			}
			if let Some(end_block) = config.end_block {
				ensure!(block_number <= end_block, Error::<T, I>::TradeEnded);
			}

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
			ensure!(bid_price >= price, Error::<T, I>::BidTooLow);

			// make deposit
			<T as pallet::Config<I>>::Currency::transfer(
				&who,
				&config.owner,
				price,
				ExistenceRequirement::KeepAlive,
			)?;

			// transfer items
			for package in bundle.clone() {
				Self::repatriate_reserved_item(
					&config.owner,
					&package.collection,
					&package.item,
					who,
					package.amount,
					ItemBalanceStatus::Free,
				)?;
			}

			// end trade
			<T as pallet::Config<I>>::Currency::unreserve(&config.owner, T::BundleDeposit::get());
			BundleOf::<T, I>::remove(trade);
			TradeConfigOf::<T, I>::remove(trade);

			Self::deposit_event(Event::<T, I>::BundleBought {
				trade: *trade,
				who: who.clone(),
				bid_price,
			});

			return Ok(())
		}
		return Err(Error::<T, I>::UnknownTrade.into())
	}

	fn do_cancel_bundle(trade: &T::TradeId, who: &T::AccountId) -> DispatchResult {
		if let Some(config) = TradeConfigOf::<T, I>::get(trade) {
			ensure!(config.trade == TradeType::Bundle, Error::<T, I>::NotBundle);

			// ensure owner
			ensure!(who.eq(&config.owner), Error::<T, I>::NoPermission);

			let bundle = BundleOf::<T, I>::get(trade);
			// unlock items
			for package in bundle.clone() {
				Self::unreserved_item(who, &package.collection, &package.item, package.amount)?;
			}

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
