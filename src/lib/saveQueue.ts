export interface SaveStatus {
  pending: boolean;
  saving: boolean;
  error: string | null;
}

export const savedStatus: SaveStatus = { pending: false, saving: false, error: null };

/** Serialize writes and keep the latest unsaved value available for retry. */
export function createSaveQueue<T>(persist: (value: T) => Promise<unknown>, notify: (status: SaveStatus) => void) {
  let revision = 0;
  let latest: T;
  let dirty = false;
  let tail: Promise<boolean> = Promise.resolve(true);

  const save = (value: T) => {
    latest = value;
    dirty = true;
    const current = ++revision;
    notify({ pending: true, saving: true, error: null });
    tail = tail.then(async () => {
      if (current !== revision) return false; // Coalesce edits not yet written.
      try {
        await persist(value);
        if (current === revision) {
          dirty = false;
          notify(savedStatus);
        }
        return true;
      } catch (error) {
        if (current === revision) {
          notify({ pending: true, saving: false, error: error instanceof Error ? error.message : String(error) });
        }
        return false;
      }
    });
  };

  return {
    save,
    retry: () => { if (dirty) save(latest); },
    isPending: () => dirty,
    async flush() {
      let current, result;
      do { current = tail; result = await current; } while (current !== tail);
      return result;
    },
  };
}
