//! Cooperative scheduling for the single HID connection. Input reads can run
//! between any output reports; heartbeat writes only run at protocol boundaries.
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(super) const QUEUE_CAPACITY: usize = 8;
pub(super) const JOB_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REPORTS: usize = 2048;
const MAX_REPORT_BYTES: usize = 1025;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(8);
const INPUT_POLL_MS: i32 = 10;

pub(super) struct IoStep {
    pub report: Vec<u8>,
    pub pause_ms: u64,
    /// True after a standalone command or the final STP of an image, never
    /// between a BAT/DRA header and its payload/commit reports.
    pub boundary_after: bool,
}

impl IoStep {
    pub fn command(report: Vec<u8>, pause_ms: u64) -> Self {
        Self { report, pause_ms, boundary_after: true }
    }
}

pub(super) fn validate_steps(steps: &[IoStep]) -> Result<(), String> {
    if steps.len() > MAX_REPORTS || steps.iter().any(|s| s.report.len() > MAX_REPORT_BYTES) {
        return Err("Deck job exceeds the output queue size limit".into());
    }
    let pauses = steps.iter().try_fold(0u64, |sum, s| sum.checked_add(s.pause_ms));
    if pauses.is_none_or(|ms| ms >= JOB_TIMEOUT.as_millis() as u64) {
        return Err("Deck job delays exceed its deadline".into());
    }
    Ok(())
}

pub(super) trait Transport {
    fn now(&self) -> Instant;
    fn write(&mut self, report: &[u8]) -> Result<(), String>;
    fn poll_input(&mut self, timeout_ms: i32) -> Result<(), String>;
    fn heartbeat(&mut self) -> Result<(), String>;
}

pub(super) fn check_deadline(now: Instant, deadline: Instant, cancelled: &AtomicBool) -> Result<(), String> {
    if cancelled.load(Ordering::Acquire) || now >= deadline {
        Err("Deck write cancelled or timed out".into())
    } else {
        Ok(())
    }
}

fn service_heartbeat(io: &mut impl Transport, last: &mut Instant, safe: bool) -> Result<(), String> {
    if safe && io.now().saturating_duration_since(*last) >= HEARTBEAT_INTERVAL {
        io.heartbeat()?;
        *last = io.now();
    }
    Ok(())
}

fn wait_responsive(
    io: &mut impl Transport, duration: Duration, safe: bool,
    deadline: Instant, cancelled: &AtomicBool, last_heartbeat: &mut Instant,
) -> Result<(), String> {
    let until = io.now() + duration;
    loop {
        check_deadline(io.now(), deadline, cancelled)?;
        service_heartbeat(io, last_heartbeat, safe)?;
        let remaining = until.saturating_duration_since(io.now());
        if remaining.is_zero() { return Ok(()); }
        let time_left = deadline.saturating_duration_since(io.now());
        let wait_ms = remaining.min(time_left).as_millis().clamp(1, INPUT_POLL_MS as u128) as i32;
        io.poll_input(wait_ms)?;
    }
}

pub(super) fn run_job(
    io: &mut impl Transport, steps: &[IoStep], initial_delay: Duration,
    deadline: Instant, cancelled: &AtomicBool, last_heartbeat: &mut Instant,
) -> Result<(), String> {
    check_deadline(io.now(), deadline, cancelled)?;
    wait_responsive(io, initial_delay, true, deadline, cancelled, last_heartbeat)?;
    let mut safe = true;
    for step in steps {
        check_deadline(io.now(), deadline, cancelled)?;
        service_heartbeat(io, last_heartbeat, safe)?;
        check_deadline(io.now(), deadline, cancelled)?;
        if !step.report.is_empty() { io.write(&step.report)?; }
        safe = step.boundary_after;
        // Zero-timeout reads between JPEG chunks avoid adding a delay per chunk.
        io.poll_input(0)?;
        wait_responsive(io, Duration::from_millis(step.pause_ms), safe,
            deadline, cancelled, last_heartbeat)?;
    }
    check_deadline(io.now(), deadline, cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeTransport<'a> {
        now: Instant,
        writes: Vec<Vec<u8>>,
        polls: Vec<i32>,
        heartbeat_at: Vec<usize>,
        write_duration: Duration,
        cancel_after_poll: bool,
        cancelled: &'a AtomicBool,
        fail_poll: bool,
    }

    impl<'a> FakeTransport<'a> {
        fn new(cancelled: &'a AtomicBool) -> Self {
            Self { now: Instant::now(), writes: Vec::new(), polls: Vec::new(),
                heartbeat_at: Vec::new(), write_duration: Duration::ZERO,
                cancel_after_poll: false, cancelled, fail_poll: false }
        }
    }

    impl Transport for FakeTransport<'_> {
        fn now(&self) -> Instant { self.now }
        fn write(&mut self, report: &[u8]) -> Result<(), String> {
            self.writes.push(report.to_vec());
            self.now += self.write_duration;
            Ok(())
        }
        fn poll_input(&mut self, timeout_ms: i32) -> Result<(), String> {
            self.polls.push(timeout_ms);
            if self.fail_poll { return Err("read disconnected".into()); }
            self.now += Duration::from_millis(timeout_ms as u64);
            if self.cancel_after_poll { self.cancelled.store(true, Ordering::Release); }
            Ok(())
        }
        fn heartbeat(&mut self) -> Result<(), String> {
            self.heartbeat_at.push(self.writes.len());
            Ok(())
        }
    }

    #[test]
    fn settle_and_packet_pauses_poll_input_in_short_slices() {
        let cancelled = AtomicBool::new(false);
        let mut io = FakeTransport::new(&cancelled);
        let deadline = io.now + JOB_TIMEOUT;
        let mut heartbeat = io.now;
        let steps = vec![IoStep::command(vec![1], 1200), IoStep::command(vec![2], 150)];
        run_job(&mut io, &steps, Duration::from_millis(1500), deadline, &cancelled, &mut heartbeat).unwrap();
        assert_eq!(io.writes, vec![vec![1], vec![2]]);
        assert!(io.polls.len() >= 285);
        assert!(io.polls.iter().all(|ms| (0..=10).contains(ms)));
    }

    #[test]
    fn reads_continue_inside_images_but_heartbeat_waits_for_commit() {
        let cancelled = AtomicBool::new(false);
        let mut io = FakeTransport::new(&cancelled);
        let deadline = io.now + JOB_TIMEOUT;
        let mut heartbeat = io.now - Duration::from_secs(7);
        io.write_duration = Duration::from_millis(500);
        let steps = vec![
            IoStep { report: b"BAT".to_vec(), pause_ms: 0, boundary_after: false },
            IoStep { report: b"JPEG-1".to_vec(), pause_ms: 0, boundary_after: false },
            IoStep { report: b"JPEG-2".to_vec(), pause_ms: 0, boundary_after: false },
            IoStep { report: b"STP".to_vec(), pause_ms: 45, boundary_after: true },
        ];
        run_job(&mut io, &steps, Duration::ZERO, deadline, &cancelled, &mut heartbeat).unwrap();
        assert_eq!(io.heartbeat_at, vec![4], "heartbeat must not appear in JPEG payload");
        assert_eq!(io.polls.iter().filter(|&&ms| ms == 0).count(), 4);
        assert_eq!(io.writes, steps.iter().map(|s| s.report.clone()).collect::<Vec<_>>());
    }

    #[test]
    fn long_wait_services_heartbeat_without_waiting_for_job_end() {
        let cancelled = AtomicBool::new(false);
        let mut io = FakeTransport::new(&cancelled);
        let deadline = io.now + JOB_TIMEOUT;
        let mut heartbeat = io.now;
        let steps = [IoStep::command(vec![1], 9000), IoStep::command(vec![2], 0)];
        run_job(&mut io, &steps, Duration::ZERO, deadline, &cancelled, &mut heartbeat).unwrap();
        assert_eq!(io.heartbeat_at, vec![1]);
    }

    #[test]
    fn expired_or_cancelled_queued_job_never_writes() {
        let cancelled = AtomicBool::new(false);
        let mut io = FakeTransport::new(&cancelled);
        let deadline = io.now;
        let mut heartbeat = io.now;
        let steps = [IoStep::command(vec![1], 0)];
        assert!(run_job(&mut io, &steps, Duration::ZERO, deadline, &cancelled, &mut heartbeat).is_err());
        cancelled.store(true, Ordering::Release);
        assert!(run_job(&mut io, &steps, Duration::ZERO, deadline + JOB_TIMEOUT, &cancelled, &mut heartbeat).is_err());
        assert!(io.writes.is_empty());
    }

    #[test]
    fn cancellation_during_a_pause_prevents_remaining_packets() {
        let cancelled = AtomicBool::new(false);
        let mut io = FakeTransport::new(&cancelled);
        io.cancel_after_poll = true;
        let deadline = io.now + JOB_TIMEOUT;
        let mut heartbeat = io.now;
        let steps = [IoStep::command(vec![1], 1200), IoStep::command(vec![2], 0)];
        assert!(run_job(&mut io, &steps, Duration::ZERO, deadline, &cancelled, &mut heartbeat).is_err());
        assert_eq!(io.writes, vec![vec![1]]);
    }

    #[test]
    fn deadline_expiring_in_a_write_or_input_error_stops_the_batch() {
        let cancelled = AtomicBool::new(false);
        let mut io = FakeTransport::new(&cancelled);
        io.write_duration = Duration::from_millis(30);
        let deadline = io.now + Duration::from_millis(20);
        let mut heartbeat = io.now;
        let steps = [IoStep::command(vec![1], 0), IoStep::command(vec![2], 0)];
        assert!(run_job(&mut io, &steps, Duration::ZERO, deadline, &cancelled, &mut heartbeat).is_err());
        assert_eq!(io.writes.len(), 1);
        let mut io = FakeTransport::new(&cancelled);
        io.fail_poll = true;
        let deadline = io.now + JOB_TIMEOUT;
        assert!(run_job(&mut io, &steps, Duration::ZERO, deadline, &cancelled, &mut heartbeat).is_err());
        assert_eq!(io.writes.len(), 1);
    }

    #[test]
    fn oversized_or_overlong_jobs_are_rejected() {
        assert!(validate_steps(&[IoStep::command(vec![0; 1026], 0)]).is_err());
        assert!(validate_steps(&[IoStep::command(vec![0], 15_000)]).is_err());
        assert!(validate_steps(&[IoStep::command(vec![0], u64::MAX), IoStep::command(vec![0], 1)]).is_err());
        let many: Vec<_> = (0..2049).map(|_| IoStep::command(vec![0], 0)).collect();
        assert!(validate_steps(&many).is_err());
    }
}
