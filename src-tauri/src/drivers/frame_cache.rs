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
#[derive(Debug, Default, Clone)]
pub struct AntiFreezeFrameCache {
    slot_hashes: HashMap<String, String>,
    pending_hashes: HashMap<String, String>,
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
        self.should_transmit_parts(device_id, slot_id, &[payload])
    }

    /// Same digest as concatenating `parts` first. The JPEG stays where it is.
    pub fn should_transmit_parts(&mut self, device_id: &str, slot_id: &str, parts: &[&[u8]]) -> Option<String> {
        self.stats.frames_evaluated += 1;
        let key = format!("{}::{}", device_id, slot_id);
        let mut hasher = Sha256::new();
        let mut len = 0u64;
        for part in parts {
            hasher.update(part);
            len += part.len() as u64;
        }
        let digest = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>();

        if let Some(prev) = self.slot_hashes.get(&key) {
            if prev == &digest {
                self.stats.frames_deduplicated_skipped += 1;
                self.stats.bytes_saved += len;
                return None;
            }
        }

        // Planning is not delivery. Pending frames must remain retryable until
        // the serialized writer explicitly completes the batch.
        self.pending_hashes.insert(key, digest.clone());
        Some(digest)
    }

    /// Complete one serialized device batch. On failure even previously cached
    /// slots are invalidated: a partial image transfer may have changed the panel.
    /// Byte counts cover image reports, excluding session/brightness commands.
    pub fn finish_transfer(&mut self, device_id: &str, success: bool, bytes: u64, elapsed_ms: u64) {
        if !success {
            self.invalidate_device(device_id);
            return;
        }
        let prefix = format!("{device_id}::");
        let keys: Vec<_> = self.pending_hashes.keys().filter(|k| k.starts_with(&prefix)).cloned().collect();
        if !keys.is_empty() {
            self.stats.last_transfer_latency_ms = elapsed_ms;
            self.stats.bytes_transmitted += bytes;
        }
        for key in keys {
            if let Some(digest) = self.pending_hashes.remove(&key) {
                self.slot_hashes.insert(key, digest);
                self.stats.frames_transmitted += 1;
            }
        }
    }

    pub fn invalidate_device(&mut self, device_id: &str) {
        let prefix = format!("{}::", device_id);
        self.slot_hashes.retain(|k, _| !k.starts_with(&prefix));
        self.pending_hashes.retain(|k, _| !k.starts_with(&prefix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planning_and_failed_batches_do_not_count_as_delivery() {
        let mut cache = AntiFreezeFrameCache::new();
        assert!(cache.should_transmit("deck", "key-0", b"image").is_some());
        assert!(cache.should_transmit("deck", "key-0", b"image").is_some());
        assert_eq!(cache.stats.frames_transmitted, 0);
        assert_eq!(cache.stats.bytes_transmitted, 0);
        cache.finish_transfer("deck", false, 1025, 100);
        assert_eq!(cache.stats.frames_transmitted, 0);
        assert_eq!(cache.stats.last_transfer_latency_ms, 0);
        assert!(cache.should_transmit("deck", "key-0", b"image").is_some());
        cache.finish_transfer("deck", true, 1025, 25);
        assert_eq!(cache.stats.frames_transmitted, 1);
        assert_eq!(cache.stats.bytes_transmitted, 1025);
        assert_eq!(cache.stats.last_transfer_latency_ms, 25);
        assert!(cache.should_transmit("deck", "key-0", b"image").is_none());
    }

    #[test]
    fn partial_failure_invalidates_only_the_affected_device() {
        let mut cache = AntiFreezeFrameCache::new();
        cache.should_transmit("a", "key-0", b"old");
        cache.should_transmit("b", "key-0", b"other");
        cache.finish_transfer("a", true, 1025, 5);
        cache.finish_transfer("b", true, 1025, 5);
        cache.should_transmit("a", "key-0", b"new");
        cache.finish_transfer("a", false, 1025, 9);
        assert!(cache.should_transmit("a", "key-0", b"old").is_some());
        assert!(cache.should_transmit("b", "key-0", b"other").is_none());
        assert_eq!(cache.stats.frames_transmitted, 2);
    }

    #[test]
    fn split_parts_hash_the_same_as_one_buffer() {
        let mut whole = AntiFreezeFrameCache::new();
        let mut parts = AntiFreezeFrameCache::new();
        let jpeg = b"jpeg-bytes";
        let extra = b"CPU 42%Cpu";
        let mut joined = jpeg.to_vec();
        joined.extend_from_slice(extra);
        assert_eq!(
            whole.should_transmit("deck", "key-0", &joined),
            parts.should_transmit_parts("deck", "key-0", &[jpeg, extra]),
        );
    }

    #[test]
    fn test_sha256_frame_deduplication_blocks_redundant_usb_writes() {
        let mut cache = AntiFreezeFrameCache::new();
        let frame_a = b"fake-jpeg-payload-key-01";
        let frame_b = b"updated-jpeg-payload-key-01";

        // 1st write transmits
        assert!(cache.should_transmit("akp05e", "key-0", frame_a).is_some());
        cache.finish_transfer("akp05e", true, 1025, 5);
        // 2nd identical write is blocked
        assert!(cache.should_transmit("akp05e", "key-0", frame_a).is_none());
        // 3rd identical write is blocked
        assert!(cache.should_transmit("akp05e", "key-0", frame_a).is_none());
        // Changed payload transmits
        assert!(cache.should_transmit("akp05e", "key-0", frame_b).is_some());
        cache.finish_transfer("akp05e", true, 1025, 6);

        assert_eq!(cache.stats.frames_evaluated, 4);
        assert_eq!(cache.stats.frames_transmitted, 2);
        assert_eq!(cache.stats.frames_deduplicated_skipped, 2);
    }
}
