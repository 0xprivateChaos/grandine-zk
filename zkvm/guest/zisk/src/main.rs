// This example program takes a number `n` as input and computes the SHA-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
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

    let phase = enum_iterator::all::<Phase>()
        .zip(0_u8..)
        .find(|(_, index)| phase_bytes.get(0) == Some(&index))
        .map(|(phase, _)| phase);

    let block = match phase {
        Some(phase) => SignedBeaconBlock::<P>::from_ssz_at_phase(phase, block_ssz)?,
        None => SignedBeaconBlock::<P>::from_ssz(config, block_ssz)?,
    };

    let state = match phase {
        Some(phase) => BeaconState::<P>::from_ssz_at_phase(phase, state_ssz)?,
        None => BeaconState::<P>::from_ssz(config, state_ssz)?,
    };

    let cache = if cache_ssz.is_empty() {
        // Build a dummy cache if none is provided, as the host would normally.
        // The real cache isn't strictly needed for the state transition function itself.
        PubkeyCache::default()
    } else {
        PubkeyCache::from_ssz(config, cache_ssz)?
    };

    Ok((block, state, cache))
}

fn main() {
    let input = read_input();

    // use Config::pectra_devnet_4() for Pectra devnet-4;
    let config = Config::pectra_devnet_6();

    let (block, mut state, cache) =
        read_block_and_state::<Mainnet>(&config, &input).expect("Failed to read input");

    state_transition(&config, &cache, &mut state, &block).expect("State transition failed");

    let root = state.hash_tree_root();

    // Write the resulting state root to the output, 4 bytes at a time.
    for i in 0..8 {
        let word = u32::from_le_bytes(root.0[i * 4..(i + 1) * 4].try_into().unwrap());
        set_output(i, word);
    }
}