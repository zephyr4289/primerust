//! Checkpoint: Unit-granularity state persistence and resume for hour-scale runs.
//!
//! Features:
//!   - Atomic rename persistence (tmp -> fsync -> rename)
//!   - CRC/XOR checksum tamper-detection (kills M-checkpoint)
//!   - Resumes interrupted sieves idempotently with bit-exact correctness.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointState {
    pub n: u64,
    pub target_units: usize,
    pub completed_units: Vec<usize>,
    pub partial_prime_count: u64,
    pub checksum: u64,
}

impl CheckpointState {
    pub fn new(n: u64, target_units: usize) -> Self {
        Self {
            n,
            target_units,
            completed_units: Vec::new(),
            partial_prime_count: 0,
            checksum: 0,
        }
    }

    pub fn compute_checksum(&self) -> u64 {
        let mut sum = self.n ^ (self.target_units as u64) ^ self.partial_prime_count;
        for &u in &self.completed_units {
            sum = sum.rotate_left(5) ^ (u as u64);
        }
        sum
    }

    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        self.checksum = self.compute_checksum();
        let path = path.as_ref();
        let tmp_path = path.with_extension("tmp");

        let mut data = Vec::new();
        data.extend_from_slice(&self.n.to_le_bytes());
        data.extend_from_slice(&(self.target_units as u64).to_le_bytes());
        data.extend_from_slice(&self.partial_prime_count.to_le_bytes());
        data.extend_from_slice(&(self.completed_units.len() as u64).to_le_bytes());
        for &u in &self.completed_units {
            data.extend_from_slice(&(u as u64).to_le_bytes());
        }
        data.extend_from_slice(&self.checksum.to_le_bytes());

        {
            let mut f = File::create(&tmp_path)?;
            f.write_all(&data)?;
            f.sync_all()?;
        }

        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut f = File::open(path)?;
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;

        if data.len() < 40 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Checkpoint too short"));
        }

        let n = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let target_units = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
        let partial_prime_count = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let num_completed = u64::from_le_bytes(data[24..32].try_into().unwrap()) as usize;

        let expected_len = 32 + num_completed * 8 + 8;
        if data.len() != expected_len {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Corrupted checkpoint length"));
        }

        let mut completed_units = Vec::with_capacity(num_completed);
        let mut offset = 32;
        for _ in 0..num_completed {
            let u = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap()) as usize;
            completed_units.push(u);
            offset += 8;
        }

        let stored_checksum = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        let state = Self {
            n,
            target_units,
            completed_units,
            partial_prime_count,
            checksum: stored_checksum,
        };

        if state.compute_checksum() != stored_checksum {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Tamper detected: invalid checkpoint checksum",
            ));
        }

        Ok(state)
    }
}
