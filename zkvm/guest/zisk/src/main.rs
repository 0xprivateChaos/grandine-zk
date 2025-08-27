#![no_main]
ziskos::entrypoint!(main);

use anyhow::Result;
use pubkey_cache::PubkeyCache;
use serde::{Deserialize, Serialize};
use ssz::{SszHash as _, SszRead as _};
use transition_functions::combined::untrusted_state_transition as state_transition;
use types::{
    combined::{BeaconState, SignedBeaconBlock},
    config::Config,
    nonstandard::Phase,
    preset::{Mainnet, Preset},
};
use ziskos::{read_input, set_output};

// #[derive(Serialize, Deserialize, Debug)]
// pub struct ZkvmInput {
//     pub state_ssz: Vec<u8>,
//     pub block_ssz: Vec<u8>,
//     pub cache_ssz: Vec<u8>,
//     pub phase: u8,
// }

/// Deserializes the input data and parses the SSZ components
fn read_block_and_state<P: Preset>(
    config: &Config,
    input: &[u8],
) -> Result<(SignedBeaconBlock<P>, BeaconState<P>, PubkeyCache)> {
    // Deserialize the input using bincode
    let (state_ssz, block_ssz, cache_ssz, phase_bytes): (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) = bincode::deserialize(input).unwrap();
    println!("Phase: {:?}", phase_bytes);
    
    // Convert phase byte to Phase enum
    let phase = enum_iterator::all::<Phase>()
        .zip(0_u8..)
        .find(|(_, index)| *index == phase_bytes[0])
        .map(|(phase, _)| phase);
    println!("Phase enum: {:?}", phase);

    // Parse the block from SSZ
    let block = match phase {
        Some(phase) => SignedBeaconBlock::<P>::from_ssz_at_phase(phase, &block_ssz)?,
        None => SignedBeaconBlock::<P>::from_ssz(config, &block_ssz)?,
    };
    println!("Block loaded");

    // Parse the state from SSZ
    let state = match phase {
        Some(phase) => BeaconState::<P>::from_ssz_at_phase(phase, &state_ssz)?,
        None => BeaconState::<P>::from_ssz(config, &state_ssz)?,
    };
    println!("State loaded");

    // Parse the cache from SSZ
    let cache = PubkeyCache::from_ssz(config, &cache_ssz)?;
    println!("Cache loaded");

    Ok((block, state, cache))
}

fn main() {
    println!("Entering the zisk guest");
    println!("Reading input");
    let input = read_input();
    println!("Input read");

    let config = Config::pectra_devnet_6();
    println!("Config loaded");

    println!("Reading block and state");
    let (block, mut state, cache) =
        read_block_and_state::<Mainnet>(&config, &input).unwrap();
    println!("Block and state read");

    println!("Performing state transition");
    state_transition(&config, &cache, &mut state, &block).unwrap();
    println!("State transition performed");

    println!("Calculating root");
    let root = state.hash_tree_root();
    println!("Root calculated");

    println!("Writing output");
    // Write the resulting state root to the output, 4 bytes at a time.
    for i in 0..8 {
        let word = u32::from_le_bytes(root.0[i * 4..(i + 1) * 4].try_into().unwrap());
        set_output(i, word);
    }
}