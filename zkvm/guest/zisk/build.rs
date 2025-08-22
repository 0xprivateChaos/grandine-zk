use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use snap::raw::Decoder;

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

    // Define paths for the consensus-spec-tests empty block transition data
    let block_path = data_dir.join("consensus-spec-tests/tests/mainnet/electra/sanity/blocks/pyspec_tests/empty_block_transition/blocks_0.ssz_snappy");
    let state_path = data_dir.join("consensus-spec-tests/tests/mainnet/electra/sanity/blocks/pyspec_tests/empty_block_transition/pre.ssz_snappy");

    // Tell cargo to rerun this script if the data files change.
    println!("cargo:rerun-if-changed={}", block_path.display());
    println!("cargo:rerun-if-changed={}", state_path.display());

    // 1) Load SSZ blobs from files and decompress snappy
    let block_compressed = fs::read(&block_path)?;
    let state_compressed = fs::read(&state_path)?;
    
    let mut decoder = Decoder::new();
    let block_ssz = decoder.decompress_vec(&block_compressed)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to decompress block: {}", e)))?;
    let state_ssz = decoder.decompress_vec(&state_compressed)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to decompress state: {}", e)))?;
    // For now, using an empty pubkey cache, as it's generated dynamically in the host.
    let cache_ssz = Vec::new();
    // phase: Electra corresponds to 5u8 (Phase::Electra enum value)
    let phase_bytes = vec![5u8];

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
