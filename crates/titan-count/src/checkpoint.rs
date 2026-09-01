//! Marathon Checkpoint: Atomic, checksummed persistence for long-running Lehmer calculations.
//!
//! Stores stage progress:
//!   - Table
//!   - P3
//!   - MT-Phi (partial sums & completed subtrees)
//!   - MT-P2 (partial sums & completed slices)
//!
//! Protocol:
//!   1. Write serialized checkpoint to `<path>.tmp`
//!   2. Flush and fsync
//!   3. Atomic rename to `<path>`
//!   4. On resume: verify CRC32, reject corrupt files, resume from last saved stage.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

fn compute_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarathonStage {
    Init = 0,
    TableReady = 1,
    P3Done = 2,
    PhiDone = 3,
    P2Done = 4,
    Complete = 5,
}

impl MarathonStage {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => MarathonStage::TableReady,
            2 => MarathonStage::P3Done,
            3 => MarathonStage::PhiDone,
            4 => MarathonStage::P2Done,
            5 => MarathonStage::Complete,
            _ => MarathonStage::Init,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MarathonState {
    pub x: u64,
    pub stage: MarathonStage,
    pub p3_val: u64,
    pub phi_val: i64,
    pub phi_completed_subtrees: usize,
    pub p2_val: u128,
    pub final_pi: u64,
}

pub struct CheckpointManager {
    pub path: PathBuf,
}

impl CheckpointManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn save(&self, state: &MarathonState) -> std::io::Result<()> {
        let tmp_path = self.path.with_extension("tmp");

        let mut data = Vec::with_capacity(128);
        data.extend_from_slice(&state.x.to_le_bytes());
        data.push(state.stage as u8);
        data.extend_from_slice(&state.p3_val.to_le_bytes());
        data.extend_from_slice(&state.phi_val.to_le_bytes());
        data.extend_from_slice(&(state.phi_completed_subtrees as u64).to_le_bytes());
        data.extend_from_slice(&state.p2_val.to_le_bytes());
        data.extend_from_slice(&state.final_pi.to_le_bytes());

        let crc = compute_crc32(&data);
        data.extend_from_slice(&crc.to_le_bytes());

        {
            let mut f = File::create(&tmp_path)?;
            f.write_all(&data)?;
            f.sync_all()?;
        }

        fs::rename(&tmp_path, &self.path)?;
        Ok(())
    }

    pub fn load(&self, expected_x: u64) -> Option<MarathonState> {
        if !self.path.exists() {
            return None;
        }

        let mut f = File::open(&self.path).ok()?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).ok()?;

        if buf.len() < 8 + 1 + 8 + 8 + 8 + 16 + 8 + 4 {
            eprintln!("[WARN] Checkpoint file truncated!");
            return None;
        }

        let payload_len = buf.len() - 4;
        let expected_crc = u32::from_le_bytes(buf[payload_len..].try_into().unwrap());
        let computed_crc = compute_crc32(&buf[..payload_len]);

        if expected_crc != computed_crc {
            eprintln!("[WARN] Checkpoint CRC mismatch! File corrupt, discarding.");
            return None;
        }

        let x = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        if x != expected_x {
            eprintln!("[WARN] Checkpoint x mismatch: stored {}, requested {}", x, expected_x);
            return None;
        }

        let stage = MarathonStage::from_u8(buf[8]);
        let p3_val = u64::from_le_bytes(buf[9..17].try_into().unwrap());
        let phi_val = i64::from_le_bytes(buf[17..25].try_into().unwrap());
        let phi_completed_subtrees = u64::from_le_bytes(buf[25..33].try_into().unwrap()) as usize;
        let p2_val = u128::from_le_bytes(buf[33..49].try_into().unwrap());
        let final_pi = u64::from_le_bytes(buf[49..57].try_into().unwrap());

        Some(MarathonState {
            x,
            stage,
            p3_val,
            phi_val,
            phi_completed_subtrees,
            p2_val,
            final_pi,
        })
    }

    pub fn clear(&self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(self.path.with_extension("tmp"));
    }
}
