#![no_main]
ziskos::entrypoint!(main);

use anyhow::Result;
use pubkey_cache::PubkeyCache;
use ssz::{SszHash as _, SszRead as _};
use transition_functions::combined::untrusted_state_transition as state_transition;
use types::{
    combined::{BeaconState, SignedBeaconBlock},
    config::Config,
    nonstandard::Phase,
    preset::{Mainnet, Preset},
};
use ziskos::{read_input, set_output};

fn read_slice<'a>(cursor: &mut usize, input: &'a [u8], len: usize) -> Result<&'a [u8]> {
    let end = *cursor + len;
    let slice = input
        .get(*cursor..end)
        .ok_or_else(|| anyhow::anyhow!("Input too short for slice"))?;
    *cursor = end;
    Ok(slice)
}

fn read_block_and_state<P: Preset>(
    config: &Config,
    input: &[u8],
) -> Result<(SignedBeaconBlock<P>, BeaconState<P>, PubkeyCache)> {
    let mut cursor = 0;

    let read_u32_len = |cursor: &mut usize, input: &[u8]| -> Result<usize> {
        let len = u32::from_le_bytes(
            input
                .get(*cursor..*cursor + 4)
                .ok_or_else(|| anyhow::anyhow!("Input too short for length"))?
                .try_into()?,
        ) as usize;
        *cursor += 4;
        Ok(len)
    };

    let state_ssz_len = read_u32_len(&mut cursor, input)?;
    let block_ssz_len = read_u32_len(&mut cursor, input)?;
    let cache_ssz_len = read_u32_len(&mut cursor, input)?;
    let phase_bytes_len = read_u32_len(&mut cursor, input)?;

    let state_ssz = read_slice(&mut cursor, input, state_ssz_len)?;
    let block_ssz = read_slice(&mut cursor, input, block_ssz_len)?;
    let cache_ssz = read_slice(&mut cursor, input, cache_ssz_len)?;
    let phase_bytes = read_slice(&mut cursor, input, phase_bytes_len)?;

    println!("Phase bytes: {:?}", phase_bytes);
    let phase = enum_iterator::all::<Phase>()
        .zip(0_u8..)
        .find(|(_, index)| phase_bytes.get(0) == Some(&index))
        .map(|(phase, _)| phase);
    println!("Phase: {:?}", phase);

    let block = match phase {
        Some(phase) => SignedBeaconBlock::<P>::from_ssz_at_phase(phase, block_ssz)?,
        None => SignedBeaconBlock::<P>::from_ssz(config, block_ssz)?,
    };
    println!("Block loaded");

    let state = match phase {
        Some(phase) => BeaconState::<P>::from_ssz_at_phase(phase, state_ssz)?,
        None => BeaconState::<P>::from_ssz(config, state_ssz)?,
    };
    println!("State loaded");

    // let cache = if cache_ssz.is_empty() {
    //     // Buildin a dummy cache
    //     // The real cache isn't strictly needed for the state transition function itself.
    //     PubkeyCache::default()
    // } else {
    //     PubkeyCache::from_ssz(config, cache_ssz)?
    // };
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
        read_block_and_state::<Mainnet>(&config, &input).expect("Failed to read input");
    println!("Block and state read");

    println!("Performing state transition");
    state_transition(&config, &cache, &mut state, &block).expect("State transition failed");
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