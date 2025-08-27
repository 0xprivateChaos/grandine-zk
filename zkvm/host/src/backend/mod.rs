use std::path::Path;
use anyhow::Result;

#[cfg(feature = "risc0")]
mod risc0;
#[cfg(feature = "risc0")]
pub use risc0::*;

#[cfg(feature = "sp1")]
mod sp1;
#[cfg(feature = "sp1")]
pub use sp1::*;

#[cfg(feature = "zisk")]
mod zisk;
#[cfg(feature = "zisk")]
pub use zisk::*;

pub trait ReportTrait {
    fn cycles(&self) -> u64;
}

pub trait ProofTrait {
    fn verify(&self) -> bool;

    fn save(&self, path: impl AsRef<Path>) -> Result<()>;
}

pub trait VmBackend: Sized {
    type Report: ReportTrait;
    type Proof: ProofTrait;

    fn new() -> Result<Self>;

    fn execute(
        &self,
        state_ssz: Vec<u8>,
        block_ssz: Vec<u8>,
        cache_ssz: Vec<u8>,
        phase_bytes: Vec<u8>,
    ) -> Result<(Vec<u8>, Self::Report)>;

    fn prove(
        &self,
        state_ssz: Vec<u8>,
        block_ssz: Vec<u8>,
        cache_ssz: Vec<u8>,
        phase_bytes: Vec<u8>,
    ) -> Result<(Vec<u8>, Self::Proof)>;
}