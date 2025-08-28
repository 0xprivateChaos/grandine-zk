use super::{ProofTrait, ReportTrait, VmBackend};
use anyhow::Result;

use std::env;
use std::path::Path;
use std::process::Command;

pub struct Report;

impl ReportTrait for Report {
    fn cycles(&self) -> u64 {
        1
    }
}

pub struct Proof;

impl ProofTrait for Proof {
    fn verify(&self) -> bool {
        true
    }

    fn save(&self, _path: impl AsRef<Path>) -> Result<()> {
        Ok(())
    }
}

pub struct Vm;

impl VmBackend for Vm {
    type Report = Report;
    type Proof = Proof;
    
    fn new() -> Result<Self> {
        Ok(Self)
    }

    fn execute(
        &self,
        state_ssz: Vec<u8>,
        block_ssz: Vec<u8>,
        cache_ssz: Vec<u8>,
        phase_bytes: Vec<u8>,
    ) -> Result<(Vec<u8>, Self::Report)> {

        // Validate input size for ZisK constraints (128MB practical limit)
        let total_input_size = state_ssz.len() + block_ssz.len() + cache_ssz.len() + phase_bytes.len();
        if total_input_size > 128 * 1024 * 1024 {
            return Err(anyhow::anyhow!(
                "Total input size {} bytes exceeds ZisK's 128MB limit",
                total_input_size
            ));
        }

        let serialized_data =
            bincode::serialize(&(state_ssz, block_ssz, cache_ssz, phase_bytes)).unwrap();

        let output_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../guest/zisk/build");
        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir)?;
        }

        let input_path = output_dir.join("input.bin");
        std::fs::write(&input_path, &serialized_data)?;

        let zisk_guest_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../guest/zisk");

        // First, build the guest program ELF file.
        let build_output = Command::new("cargo-zisk")
            .arg("build")
            .arg("--release")
            .current_dir(&zisk_guest_dir)
            .output()?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to build zisk guest program. Stderr: {}",
                stderr
            ));
        }

        // Second, execute the ELF file using ziskemu with a high step count.
        let elf_path = zisk_guest_dir
            .join("../../../target/riscv64ima-zisk-zkvm-elf/release/zkvm_guest_zisk");
        let output = Command::new("ziskemu")
            .arg("-e")
            .arg(elf_path)
            .arg("-i")
            .arg(input_path)
            .arg("--max-steps")
            .arg("100000000000000")
            .current_dir(&zisk_guest_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to execute ziskemu command. Stderr: {}",
                stderr
            ));
        }

        // Parse the 8 u32 outputs from the guest program
        let outputs = self.parse_ziskemu_outputs(&output.stdout)?;
        
        if outputs.len() != 8 {
            return Err(anyhow::anyhow!(
                "Expected 8 outputs from guest program, but got {}. Full output:\n{}",
                outputs.len(),
                String::from_utf8_lossy(&output.stdout)
            ));
        }

        // Convert the 8 u32 outputs back to a 256-bit state root
        let state_root = self.reconstruct_state_root(&outputs)?;
        
        Ok((state_root, Report))
    }
    
    fn prove(
        &self,
        _state_ssz: Vec<u8>,
        _block_ssz: Vec<u8>,
        _cache_ssz: Vec<u8>,
        _phase_bytes: Vec<u8>,
    ) -> Result<(Vec<u8>, Self::Proof)> {
        Ok((vec![], Proof))
    }
}

impl Vm {
    /// Parse the 8 u32 outputs from ziskemu stdout
    fn parse_ziskemu_outputs(&self, stdout: &[u8]) -> Result<Vec<u32>, anyhow::Error> {
        let stdout_str = String::from_utf8_lossy(stdout);
        let mut outputs = Vec::new();
        
        // Process only the lines that are valid 8-character hex strings
        for line in stdout_str.lines() {
            let trimmed = line.trim();
            if trimmed.len() == 8 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                // Parse the hex string as u32 (little-endian as output by guest)
                let value = u32::from_str_radix(trimmed, 16)?;
                outputs.push(value);
            }
        }
        
        Ok(outputs)
    }
    
    /// Reconstruct the 256-bit state root from 8 u32 outputs
    fn reconstruct_state_root(&self, outputs: &[u32]) -> Result<Vec<u8>, anyhow::Error> {
        if outputs.len() != 8 {
            return Err(anyhow::anyhow!("Expected 8 outputs, got {}", outputs.len()));
        }
        
        let mut state_root_bytes = Vec::with_capacity(32);
        
        // Convert each u32 to 4 bytes (little-endian) and append
        for &output in outputs {
            state_root_bytes.extend_from_slice(&output.to_le_bytes());
        }
        
        // Verify we have exactly 32 bytes (256 bits)
        if state_root_bytes.len() != 32 {
            return Err(anyhow::anyhow!(
                "State root reconstruction failed: expected 32 bytes, got {}",
                state_root_bytes.len()
            ));
        }
        
        Ok(state_root_bytes)
    }
}