use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrameCacheStats {
    pub frames_evaluated: u64,
    pub frames_transmitted: u64,
    pub frames_deduplicated_skipped: u64,
    pub bytes_transmitted: u64,
    pub bytes_saved: u64,
    pub last_transfer_latency_ms: u64,
}

/// Prevents the official Stream Dock AJAZZ app's 1-3s redraw flood (`sigTooMuchDataWaitingToBeSent`)
/// by storing per-device, per-slot SHA-256 digests of the last transmitted JPEG frame.
#[derive(Debug, Default)]
pub struct AntiFreezeFrameCache {
    slot_hashes: HashMap<String, String>,
    pub stats: FrameCacheStats,
}

impl AntiFreezeFrameCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compute_sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Returns `Some(sha256_hex)` if the payload has changed and MUST be sent over USB,
    /// or `None` if the exact frame is already displayed on the hardware key/strip.
    pub fn should_transmit(&mut self, device_id: &str, slot_id: &str, payload: &[u8]) -> Option<String> {
        self.stats.frames_evaluated += 1;
        let key = format!("{}::{}", device_id, slot_id);
        let digest = Self::compute_sha256_hex(payload);

        if let Some(prev) = self.slot_hashes.get(&key) {
            if prev == &digest {
                self.stats.frames_deduplicated_skipped += 1;
                self.stats.bytes_saved += payload.len() as u64;
                return None;
            }
        }

        self.slot_hashes.insert(key, digest.clone());
        self.stats.frames_transmitted += 1;
        self.stats.bytes_transmitted += payload.len() as u64;
        Some(digest)
    }

    pub fn invalidate_device(&mut self, device_id: &str) {
        let prefix = format!("{}::", device_id);
        self.slot_hashes.retain(|k, _| !k.starts_with(&prefix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_frame_deduplication_blocks_redundant_usb_writes() {
        let mut cache = AntiFreezeFrameCache::new();
        let frame_a = b"fake-jpeg-payload-key-01";
        let frame_b = b"updated-jpeg-payload-key-01";

        // 1st write transmits
        assert!(cache.should_transmit("akp05e", "key-0", frame_a).is_some());
        // 2nd identical write is blocked
        assert!(cache.should_transmit("akp05e", "key-0", frame_a).is_none());
        // 3rd identical write is blocked
        assert!(cache.should_transmit("akp05e", "key-0", frame_a).is_none());
        // Changed payload transmits
        assert!(cache.should_transmit("akp05e", "key-0", frame_b).is_some());

        assert_eq!(cache.stats.frames_evaluated, 4);
        assert_eq!(cache.stats.frames_transmitted, 2);
        assert_eq!(cache.stats.frames_deduplicated_skipped, 2);
    }
}
