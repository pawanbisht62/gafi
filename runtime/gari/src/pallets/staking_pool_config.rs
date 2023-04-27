use gafi_support::{
	common::{
		constant::ID,
		currency::{unit, NativeToken::GAFI},
	},
	pool::{SystemDefaultServices, SystemService, SystemServicePack},
};
use sp_runtime::Permill;

use crate::{Balances, Runtime, RuntimeEvent};
use codec::{Encode, Decode};
use sp_std::vec;
use scale_info::TypeInfo;

const STAKING_BASIC_ID: ID = [0_u8; 32];
const STAKING_MEDIUM_ID: ID = [1_u8; 32];
const STAKING_ADVANCE_ID: ID = [2_u8; 32];

impl SystemDefaultServices for StakingPoolDefaultServices {
	fn get_default_services() -> SystemServicePack {
		SystemServicePack::new(vec![
			(
				STAKING_BASIC_ID,
				SystemService::new(
					STAKING_BASIC_ID,
					100_u32,
					Permill::from_percent(10),
					1000 * unit(GAFI),
				),
			),
			(
				STAKING_MEDIUM_ID,
				SystemService::new(
					STAKING_MEDIUM_ID,
					100_u32,
					Permill::from_percent(20),
					2000 * unit(GAFI),
				),
			),
			(
				STAKING_ADVANCE_ID,
				SystemService::new(
					STAKING_ADVANCE_ID,
					100_u32,
					Permill::from_percent(30),
					3000 * unit(GAFI),
				),
			),
		])
	}
}

#[derive(Eq, PartialEq, Clone, Encode, Decode, Debug, TypeInfo, Default)]
pub struct StakingPoolDefaultServices {}

impl staking_pool::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type WeightInfo = staking_pool::weights::SubstrateWeight<Runtime>;
	type StakingServices = StakingPoolDefaultServices;
}
