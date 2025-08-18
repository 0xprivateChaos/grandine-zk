use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

// Define constants for the directory and input file name
const OUTPUT_DIR: &str = "build/";
const FILE_NAME: &str = "input.bin";

fn main() -> io::Result<()> {
    // Base directory for data files, assuming zkvm/data layout
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let zkvm_dir = Path::new(&manifest_dir)
        .parent()
        .and_then(Path::parent)
        .expect("Could not find zkvm directory from guest manifest path");
    let data_dir = zkvm_dir.join("data");

    // Define paths for the specific test case data
    let block_path = data_dir.join("pectra-devnet-6/beacon_block_slot_00021568_root_0xb28a634b89c669141990ed5deceb1ea4777869a64cb8eaccb6cb9f4796c5110d.ssz");
    let state_path = data_dir.join("pectra-devnet-6/beacon_state_slot_00021567_root_0xd51b605669c3e1ec96d83b6ab191d921f276d363621009fa6fd4a171a6bbf943.ssz");

    // Tell cargo to rerun this script if the data files change.
    println!("cargo:rerun-if-changed={}", block_path.display());
    println!("cargo:rerun-if-changed={}", state_path.display());

    // 1) Load SSZ blobs from files
    let block_ssz = fs::read(&block_path)?;
    let state_ssz = fs::read(&state_path)?;
    // For now, using an empty pubkey cache, as it's generated dynamically in the host.
    let cache_ssz = Vec::new();
    // phase: None corresponds to 255u8 in the host code for tests without a specific phase.
    let phase_bytes = vec![255u8];

    // 2) Prepare buffer: [4x u32 lengths] + [concatenated SSZ bytes]
    let mut buf = Vec::with_capacity(
        16 + state_ssz.len() + block_ssz.len() + cache_ssz.len() + phase_bytes.len(),
    );

    // Helper to write length as u32 little-endian
    let write_len = |len: usize, out: &mut Vec<u8>| {
        out.extend_from_slice(&(len as u32).to_le_bytes());
    };

    write_len(state_ssz.len(), &mut buf);
    write_len(block_ssz.len(), &mut buf);
    write_len(cache_ssz.len(), &mut buf);
    write_len(phase_bytes.len(), &mut buf);

    buf.extend_from_slice(&state_ssz);
    buf.extend_from_slice(&block_ssz);
    buf.extend_from_slice(&cache_ssz);
    buf.extend_from_slice(&phase_bytes);

    // 3) Write the buffer to zkvm/guest/zisk/build/input.bin
    let output_dir = Path::new(OUTPUT_DIR);
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }
    let file_path = output_dir.join(FILE_NAME);
    let mut file = File::create(&file_path)?;
    file.write_all(&buf)?;

    Ok(())
}
