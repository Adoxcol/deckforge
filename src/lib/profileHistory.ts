/** Editor dirty-state and undo without serializing the whole profile library. */

export type ProfileClock = { current: number; saved: number };

export type HistoryEntry<T> = { revision: number; state: T };

export function initialClock(): ProfileClock {
  return { current: 0, saved: 0 };
}

export function isClean(clock: ProfileClock): boolean {
  return clock.current === clock.saved;
}

/**
 * New array, same profile objects. Safe because edits replace a profile instead
 * of writing into the object already stored here.
 */
export function captureLibrary<T>(profiles: readonly T[]): T[] {
  return profiles.slice();
}

export function pushEntry<T>(stack: readonly HistoryEntry<T>[], entry: HistoryEntry<T>, limit = 20): HistoryEntry<T>[] {
  const next = stack.length >= limit ? stack.slice(stack.length - (limit - 1)) : stack.slice();
  next.push(entry);
  return next;
}

/** Revision to store on the undo entry, and the clock after this edit. */
export function beginEdit(clock: ProfileClock): { undoRevision: number; clock: ProfileClock } {
  return { undoRevision: clock.current, clock: { current: clock.current + 1, saved: clock.saved } };
}

/** A save that started at `revision` finished. Later edits stay dirty. */
export function noteSaved(clock: ProfileClock, revision: number): ProfileClock {
  return { current: clock.current, saved: revision };
}

export function noteRestored(clock: ProfileClock, revision: number): ProfileClock {
  return { current: revision, saved: clock.saved };
}
