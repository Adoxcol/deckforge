/** Capture the exact saved revision and serialize explicit profile writes. */
export function profileSnapshot(value: unknown): string { return JSON.stringify(value); }

export function createProfilePersistence<T>(persist: (value: T) => Promise<unknown>, saved: (snapshot: string) => void) {
  let tail: Promise<unknown> = Promise.resolve();
  let pending = 0;
  let savedRevision: string | null = null;
  return {
    save(value: T) {
      const snapshot = profileSnapshot(value);
      const copy: T = JSON.parse(snapshot);
      pending++;
      const result = tail.then(async () => { await persist(copy); savedRevision = snapshot; saved(snapshot); });
      tail = result.catch(() => {}).finally(() => { pending--; });
      return result;
    },
    async flush() { let current; do { current = tail; await current; } while (current !== tail); },
    isPending: () => pending > 0,
    savedSnapshot: () => savedRevision,
  };
}
