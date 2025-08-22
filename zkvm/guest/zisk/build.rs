use std::env;
use std::fs;
use std::io;
use std::path::Path;
use snap::raw::Decoder;
use serde::{Deserialize, Serialize};

// Define constants for the directory and input file name
const OUTPUT_DIR: &str = "build/";
const FILE_NAME: &str = "input.bin";

#[derive(Serialize, Deserialize, Debug)]
pub struct ZkvmInput {
    pub state_ssz: Vec<u8>,
    pub block_ssz: Vec<u8>,
    pub cache_ssz: Vec<u8>,
    pub phase: u8,
}


/// Loads and decompresses a snappy-compressed file
fn load_and_decompress_file(path: &Path) -> io::Result<Vec<u8>> {
    let compressed_data = fs::read(path)?;
    let mut decoder = Decoder::new();
    decoder
        .decompress_vec(&compressed_data)
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to decompress {}: {}", path.display(), e),
            )
        })
}

/// Constructs the data directory path relative to the zkvm directory
fn get_data_directory() -> io::Result<std::path::PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("CARGO_MANIFEST_DIR not set: {}", e)))?;
    
    let zkvm_dir = Path::new(&manifest_dir)
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Could not find zkvm directory from guest manifest path"
            )
        })?;
    
    Ok(zkvm_dir.join("data"))
}

/// Creates the output directory if it doesn't exist and returns the full output file path
fn ensure_output_path() -> io::Result<std::path::PathBuf> {
    let output_dir = Path::new(OUTPUT_DIR);
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }
    Ok(output_dir.join(FILE_NAME))
}

fn main() -> io::Result<()> {
    // Set up paths
    let data_dir = get_data_directory()?;
    
    // Define paths for the consensus-spec-tests empty block transition data
    let test_base_path = "consensus-spec-tests/tests/mainnet/electra/sanity/blocks/pyspec_tests/empty_block_transition";
    let block_path = data_dir.join(format!("{}/blocks_0.ssz_snappy", test_base_path));
    let state_path = data_dir.join(format!("{}/pre.ssz_snappy", test_base_path));

    // Tell cargo to rerun this script if the data files change
    println!("cargo:rerun-if-changed={}", block_path.display());
    println!("cargo:rerun-if-changed={}", state_path.display());

    // Load and decompress the SSZ data
    let block_ssz = load_and_decompress_file(&block_path)?;
    let state_ssz = load_and_decompress_file(&state_path)?;
    
    // Create the input data structure
    let zkvm_input = ZkvmInput {
        state_ssz,
        block_ssz,
        cache_ssz: Vec::new(), // Empty for now, generated dynamically in the host
        phase: 5, // Electra phase
    };

    // Serialize using bincode
    let serialized_data = bincode::serialize(&zkvm_input).unwrap();
    
    // Write to output file
    let output_path = ensure_output_path()?;
    fs::write(&output_path, &serialized_data)?;

    println!("Successfully generated {} ({} bytes)", output_path.display(), serialized_data.len());
    
    Ok(())
}
