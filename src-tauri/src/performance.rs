//! Bounded, process-local timings. No user content or disk writes.
use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
const WINDOW: usize = 128;
#[derive(Clone, Copy)]
pub(crate) enum Stage { ActionQueue, Action, Media, Telemetry, FullRender, FaceRender, Animation, UsbQueue, UsbWrite }
const LABELS: [&str; 9] = ["Action queue wait", "Action handling", "Windows media polling", "Telemetry scan", "Full layout rendering", "Live image rendering", "Animation packet assembly", "USB queue wait", "USB job execution"];
struct Samples { values: [f64; WINDOW], count: u64, next: usize, len: usize }
impl Default for Samples {
    fn default() -> Self { Self { values: [0.0; WINDOW], count: 0, next: 0, len: 0 } }
}
impl Samples {
    fn record(&mut self, elapsed: Duration) {
        self.values[self.next] = elapsed.as_secs_f64() * 1000.0;
        self.next = (self.next + 1) % WINDOW;
        self.len = (self.len + 1).min(WINDOW);
        self.count = self.count.saturating_add(1);
    }
    fn summary(&self, label: &str) -> Timing {
        let mut sorted = self.values[..self.len].to_vec();
        sorted.sort_by(f64::total_cmp);
        let percentile = |percent: usize| {
            if sorted.is_empty() { None } else { Some(sorted[(sorted.len() * percent).div_ceil(100) - 1]) }
        };
        Timing { label: label.into(), count: self.count, samples: self.len,
            last_ms: if self.len == 0 { None } else { Some(self.values[(self.next + WINDOW - 1) % WINDOW]) },
            median_ms: percentile(50), p95_ms: percentile(95), max_ms: sorted.last().copied() }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Timing {
    label: String, count: u64, samples: usize,
    last_ms: Option<f64>, median_ms: Option<f64>, p95_ms: Option<f64>, max_ms: Option<f64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot { window_size: usize, elapsed_seconds: f64, timings: Vec<Timing> }
struct Store { started: Instant, stages: [Samples; 9] }
impl Default for Store {
    fn default() -> Self { Self { started: Instant::now(), stages: std::array::from_fn(|_| Samples::default()) } }
}
fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Store::default()))
}
pub(crate) fn record(stage: Stage, elapsed: Duration) {
    store().lock().unwrap_or_else(|e| e.into_inner()).stages[stage as usize].record(elapsed);
}
pub(crate) struct Timer { stage: Stage, started: Instant }
pub(crate) fn timer(stage: Stage) -> Timer { Timer { stage, started: Instant::now() } }
impl Drop for Timer {
    fn drop(&mut self) { record(self.stage, self.started.elapsed()); }
}
pub(crate) fn snapshot(reset: bool) -> Snapshot {
    let mut guard = store().lock().unwrap_or_else(|e| e.into_inner());
    if reset { *guard = Store::default(); }
    Snapshot { window_size: WINDOW, elapsed_seconds: guard.started.elapsed().as_secs_f64(),
        timings: guard.stages.iter().zip(LABELS).map(|(samples, label)| samples.summary(label)).collect() }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn percentiles_use_recent_bounded_samples_and_keep_total_count() {
        let mut samples = Samples::default();
        for i in 1..=256 { samples.record(Duration::from_millis(i)); }
        let result = samples.summary("test");
        assert_eq!(result.count, 256);
        assert_eq!(result.samples, 128);
        assert_eq!(result.last_ms, Some(256.0));
        assert_eq!(result.median_ms, Some(192.0));
        assert_eq!(result.p95_ms, Some(250.0));
        assert_eq!(result.max_ms, Some(256.0));
    }
    #[test]
    fn empty_timings_are_absent_and_submillisecond_samples_are_retained() {
        let mut samples = Samples::default();
        assert_eq!(samples.summary("test").max_ms, None);
        samples.record(Duration::from_micros(125));
        let result = samples.summary("test");
        assert_eq!(result.last_ms, Some(0.125));
        assert_eq!(result.p95_ms, Some(0.125));
        assert_eq!(Store::default().stages[0].summary("test").count, 0);
    }
}
