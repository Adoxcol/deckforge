//! Decides whether an LCD key tap, double-press, or hold should run.
//!
//! A key with only a tap action fires on the way down, the same as before.
//! Hold fires once the key has stayed down. Double-press waits after the
//! release so a single tap can still be told apart.

pub(crate) const HOLD_MS: u64 = 450;
pub(crate) const DOUBLE_MS: u64 = 280;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GestureKind {
    Tap,
    Double,
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GestureBindings {
    pub has_double: bool,
    pub has_hold: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GestureEdge {
    Down,
    Up,
}

#[derive(Clone, Copy)]
struct Slot {
    key: u8,
    down_at: u64,
    phase: Phase,
}

#[derive(Clone, Copy)]
enum Phase {
    Held,
    HeldDone,
    WaitSecond { up_at: u64 },
}

#[derive(Default)]
pub(crate) struct GestureTracker {
    slots: Vec<Slot>,
}

impl GestureTracker {
    pub(crate) fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub(crate) fn edge(&mut self, key: u8, edge: GestureEdge, at: u64, bindings: GestureBindings) -> Vec<(u8, GestureKind)> {
        if !bindings.has_double && !bindings.has_hold {
            self.slots.retain(|slot| slot.key != key);
            return if edge == GestureEdge::Down { vec![(key, GestureKind::Tap)] } else { Vec::new() };
        }
        match edge {
            GestureEdge::Down => self.down(key, at, bindings),
            GestureEdge::Up => self.up(key, at, bindings),
        }
    }

    pub(crate) fn poll(&mut self, now: u64, mut bindings_for: impl FnMut(u8) -> GestureBindings) -> Vec<(u8, GestureKind)> {
        let mut fired = Vec::new();
        let mut index = 0;
        while index < self.slots.len() {
            let slot = self.slots[index];
            let bindings = bindings_for(slot.key);
            match slot.phase {
                Phase::Held if bindings.has_hold && now.saturating_sub(slot.down_at) >= HOLD_MS => {
                    self.slots[index].phase = Phase::HeldDone;
                    fired.push((slot.key, GestureKind::Hold));
                    index += 1;
                }
                Phase::WaitSecond { up_at } if !bindings.has_double || now.saturating_sub(up_at) > DOUBLE_MS => {
                    self.slots.remove(index);
                    fired.push((slot.key, GestureKind::Tap));
                }
                _ => index += 1,
            }
        }
        fired
    }

    fn down(&mut self, key: u8, at: u64, bindings: GestureBindings) -> Vec<(u8, GestureKind)> {
        let mut fired = Vec::new();
        if let Some(pos) = self.slots.iter().position(|slot| slot.key == key) {
            match self.slots[pos].phase {
                Phase::WaitSecond { up_at } if bindings.has_double && at.saturating_sub(up_at) <= DOUBLE_MS => {
                    self.slots.remove(pos);
                    return vec![(key, GestureKind::Double)];
                }
                Phase::WaitSecond { .. } => {
                    self.slots.remove(pos);
                    fired.push((key, GestureKind::Tap));
                }
                Phase::Held | Phase::HeldDone => return Vec::new(),
            }
        }
        self.slots.push(Slot { key, down_at: at, phase: Phase::Held });
        fired
    }

    fn up(&mut self, key: u8, at: u64, bindings: GestureBindings) -> Vec<(u8, GestureKind)> {
        let Some(pos) = self.slots.iter().position(|slot| slot.key == key) else {
            return Vec::new();
        };
        match self.slots[pos].phase {
            Phase::HeldDone => {
                self.slots.remove(pos);
                Vec::new()
            }
            Phase::Held if bindings.has_hold && at.saturating_sub(self.slots[pos].down_at) >= HOLD_MS => {
                self.slots.remove(pos);
                vec![(key, GestureKind::Hold)]
            }
            Phase::Held if bindings.has_double => {
                self.slots[pos].phase = Phase::WaitSecond { up_at: at };
                Vec::new()
            }
            Phase::Held => {
                self.slots.remove(pos);
                vec![(key, GestureKind::Tap)]
            }
            Phase::WaitSecond { .. } => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn both() -> GestureBindings {
        GestureBindings { has_double: true, has_hold: true }
    }

    #[test]
    fn a_plain_key_fires_on_the_way_down() {
        let mut tracker = GestureTracker::new();
        let plain = GestureBindings { has_double: false, has_hold: false };
        assert_eq!(tracker.edge(0, GestureEdge::Down, 0, plain), vec![(0, GestureKind::Tap)]);
        assert!(tracker.edge(0, GestureEdge::Up, 40, plain).is_empty());
    }

    #[test]
    fn hold_fires_once_and_does_not_also_tap() {
        let mut tracker = GestureTracker::new();
        let hold = GestureBindings { has_double: false, has_hold: true };
        assert!(tracker.edge(1, GestureEdge::Down, 0, hold).is_empty());
        assert!(tracker.poll(HOLD_MS - 1, |_| hold).is_empty());
        assert_eq!(tracker.poll(HOLD_MS, |_| hold), vec![(1, GestureKind::Hold)]);
        assert!(tracker.poll(HOLD_MS + 500, |_| hold).is_empty());
        assert!(tracker.edge(1, GestureEdge::Up, HOLD_MS + 500, hold).is_empty());
    }

    #[test]
    fn a_short_press_on_a_hold_key_is_still_a_tap() {
        let mut tracker = GestureTracker::new();
        let hold = GestureBindings { has_double: false, has_hold: true };
        tracker.edge(1, GestureEdge::Down, 0, hold);
        assert_eq!(tracker.edge(1, GestureEdge::Up, HOLD_MS - 1, hold), vec![(1, GestureKind::Tap)]);
    }

    #[test]
    fn releasing_after_the_hold_window_without_a_poll_still_holds() {
        let mut tracker = GestureTracker::new();
        let hold = GestureBindings { has_double: false, has_hold: true };
        tracker.edge(1, GestureEdge::Down, 0, hold);
        assert_eq!(tracker.edge(1, GestureEdge::Up, HOLD_MS, hold), vec![(1, GestureKind::Hold)]);
    }

    #[test]
    fn double_press_waits_before_firing_the_tap() {
        let mut tracker = GestureTracker::new();
        let double = GestureBindings { has_double: true, has_hold: false };
        assert!(tracker.edge(2, GestureEdge::Down, 0, double).is_empty());
        assert!(tracker.edge(2, GestureEdge::Up, 40, double).is_empty());
        assert!(tracker.poll(40 + DOUBLE_MS, |_| double).is_empty());
        assert_eq!(tracker.poll(41 + DOUBLE_MS, |_| double), vec![(2, GestureKind::Tap)]);
    }

    #[test]
    fn a_second_press_inside_the_window_is_a_double_and_not_a_tap() {
        let mut tracker = GestureTracker::new();
        let double = GestureBindings { has_double: true, has_hold: false };
        tracker.edge(2, GestureEdge::Down, 0, double);
        tracker.edge(2, GestureEdge::Up, 40, double);
        assert_eq!(tracker.edge(2, GestureEdge::Down, 40 + DOUBLE_MS, double), vec![(2, GestureKind::Double)]);
        assert!(tracker.edge(2, GestureEdge::Up, 100 + DOUBLE_MS, double).is_empty());
        assert!(tracker.poll(1_000, |_| double).is_empty());
    }

    #[test]
    fn hold_wins_when_the_key_also_has_a_double_press() {
        let mut tracker = GestureTracker::new();
        assert!(tracker.edge(3, GestureEdge::Down, 0, both()).is_empty());
        assert_eq!(tracker.poll(HOLD_MS, |_| both()), vec![(3, GestureKind::Hold)]);
        assert!(tracker.edge(3, GestureEdge::Up, HOLD_MS + 10, both()).is_empty());
    }

    #[test]
    fn another_key_does_not_cancel_a_waiting_tap() {
        let mut tracker = GestureTracker::new();
        let double = GestureBindings { has_double: true, has_hold: false };
        tracker.edge(0, GestureEdge::Down, 0, double);
        tracker.edge(0, GestureEdge::Up, 20, double);
        assert_eq!(tracker.edge(4, GestureEdge::Down, 30, GestureBindings { has_double: false, has_hold: false }), vec![(4, GestureKind::Tap)]);
        assert_eq!(tracker.poll(21 + DOUBLE_MS, |_| double), vec![(0, GestureKind::Tap)]);
    }
}
