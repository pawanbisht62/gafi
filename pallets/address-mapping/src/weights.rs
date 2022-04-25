//! Autogenerated weights for `gafi-tx`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-03-28, STEPS: `20`, REPEAT: 10, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/gafi-node
// benchmark
// --chain
// dev
// --execution
// wasm
// --wasm-execution
// compiled
// --pallet
// gafi-tx
// --extrinsic
// *
// --steps
// 20
// --repeat
// 10
// --json-file=raw.json
// --output
// ./pallets/src/gafi-tx/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
	fn bond(s: u32,) -> Weight;
	fn unbond(s: u32,) -> Weight;
}

/// Weight functions for `upfront_pool`.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: TxHandler H160Mapping (r:1 w:1)
	// Storage: TxHandler Id32Mapping (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn bond(s: u32, ) -> Weight {
		172_333_000_u64
			// Standard Error: 636_000
			.saturating_add((3_833_000_u64).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(8_u64 ))
			.saturating_add(T::DbWeight::get().writes(6_u64 ))
	}
	// Storage: TxHandler Id32Mapping (r:1 w:1)
	// Storage: TxHandler H160Mapping (r:1 w:1)
	fn unbond(s: u32, ) -> Weight {
		(8_000_000_u64 )
			// Standard Error: 166_000
			.saturating_add((167_000_u64 ).saturating_mul(s.into() ))
			.saturating_add(T::DbWeight::get().reads(2_u64 ))
			.saturating_add(T::DbWeight::get().writes(2_u64 ))
	}
}

impl WeightInfo for () {
	
	fn bond(s: u32, ) -> Weight {
		(172_333_000_u64 )
		// Standard Error: 636_000
		.saturating_add((3_833_000_u64 ).saturating_mul(s.into()))
		.saturating_add(RocksDbWeight::get().reads(8_u64 ))
		.saturating_add(RocksDbWeight::get().writes(6_u64 ))
	}

	fn unbond(s: u32, ) -> Weight {
		(8_000_000_u64 )
			// Standard Error: 166_000
			.saturating_add((167_000_u64 ).saturating_mul(s.into()))
			.saturating_add(RocksDbWeight::get().reads(2_u64 ))
			.saturating_add(RocksDbWeight::get().writes(2_u64 ))
	}
}
