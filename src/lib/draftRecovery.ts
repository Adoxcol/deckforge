export interface DraftStatus { pending: boolean; error: string | null }

/** Debounce draft snapshots, serialize writes/clears, and retain failures for retry. */
export function createDraftRecovery<T>(persist: (value: T | null) => Promise<void>, notify: (status: DraftStatus) => void, delay = 500) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let tail: Promise<boolean> = Promise.resolve(true);
  let latest: T | null = null;
  let revision = 0;
  let paused = false;
  const enqueue = (value: T | null, current: number) => {
    tail = tail.then(async () => {
      if (current !== revision) return false;
      try {
        await persist(value);
        if (current === revision) notify({ pending: false, error: null });
        return true;
      } catch (error) {
        if (current === revision) notify({ pending: true, error: String(error instanceof Error ? error.message : error) });
        return false;
      }
    });
    return tail;
  };
  const schedule = (value: T | null) => {
    if (paused) return;
    latest = value === null ? null : JSON.parse(JSON.stringify(value));
    clearTimeout(timer);
    const current = ++revision;
    notify({ pending: true, error: null });
    timer = setTimeout(() => { timer = undefined; void enqueue(latest, current); }, delay);
  };
  return {
    schedule,
    retry: () => schedule(latest),
    pause: () => { paused = true; clearTimeout(timer); timer = undefined; ++revision; },
    resume: () => { paused = false; },
    async clear() {
      clearTimeout(timer); timer = undefined; latest = null;
      const current = ++revision;
      notify({ pending: true, error: null });
      return enqueue(null, current);
    },
    async flush() {
      if (timer !== undefined) { clearTimeout(timer); timer = undefined; void enqueue(latest, revision); }
      let current, result;
      do { current = tail; result = await current; } while (current !== tail);
      return result;
    },
  };
}
