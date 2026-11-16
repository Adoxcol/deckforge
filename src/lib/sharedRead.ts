/** Share concurrent reads and briefly reuse successes; errors are never cached. */
export function createSharedRead<T>(read: () => Promise<T>, maxAgeMs: number, now = () => performance.now()) {
  let pending: Promise<T> | null = null;
  let cached: { value: T; at: number } | null = null;
  return (force = false): Promise<T> => {
    if (pending) return pending;
    if (!force && cached && now() - cached.at < maxAgeMs) return Promise.resolve(cached.value);
    pending = Promise.resolve().then(read).then(value => {
      cached = { value, at: now() };
      return value;
    }, error => {
      cached = null;
      throw error;
    }).finally(() => { pending = null; });
    return pending;
  };
}
