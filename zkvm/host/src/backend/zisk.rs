use super::{ProofTrait, ReportTrait, VmBackend};
use anyhow::Result;
use ziskos::{read_input, set_output};

use std::path::Path;

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

    fn save(&self, path: impl AsRef<Path>) -> Result<()> {
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
        _state_ssz: Vec<u8>,
        _block_ssz: Vec<u8>,
        _cache_ssz: Vec<u8>,
        _phase_bytes: Vec<u8>,
    ) -> Result<(Vec<u8>, Self::Report)> {
        Ok((vec![], Report))
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