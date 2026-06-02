//! Persistent discovery checkpoint. Stores the discovered borrower set and the
//! last block scanned so a restart resumes instead of re-backfilling. Writes
//! are atomic (write to a temp file, then rename) so a crash mid-write can't
//! corrupt the checkpoint.

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryCheckpoint {
    pub last_block: u64,
    pub borrowers: Vec<Address>,
}

impl DiscoveryCheckpoint {
    /// Load a checkpoint, or `None` if the file doesn't exist yet.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&raw)?))
    }

    /// Atomically persist via temp-file + rename.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("checkpoint.json");

        let cp = DiscoveryCheckpoint {
            last_block: 42_000_000,
            borrowers: vec![Address::with_last_byte(1), Address::with_last_byte(2)],
        };
        cp.save(&path).expect("save");

        let loaded = DiscoveryCheckpoint::load(&path).expect("load").expect("some");
        assert_eq!(loaded, cp);
    }

    #[test]
    fn missing_file_is_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nope.json");
        assert!(DiscoveryCheckpoint::load(&path).expect("load").is_none());
    }

    #[test]
    fn save_overwrites() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cp.json");
        DiscoveryCheckpoint { last_block: 1, borrowers: vec![] }
            .save(&path)
            .expect("save");
        let updated = DiscoveryCheckpoint {
            last_block: 2,
            borrowers: vec![Address::with_last_byte(9)],
        };
        updated.save(&path).expect("save");
        assert_eq!(
            DiscoveryCheckpoint::load(&path).expect("load").expect("some"),
            updated
        );
    }
}
