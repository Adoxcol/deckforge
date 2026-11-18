/** Share icon lookups across every rule tile. Missing icons are retried later. */
export const createAppIconLoader = (
  fetchIcon: (exe: string) => Promise<string | null>,
  now: () => number = Date.now,
  missRetryMs = 5 * 60_000,
) => {
  const resolved = new Map<string, { url: string | null; checkedAt: number }>();
  const pending = new Map<string, Promise<string | null>>();

  return (exe: string): Promise<string | null> => {
    const key = exe.trim().toLowerCase();
    if (!key) return Promise.resolve(null);
    const cached = resolved.get(key);
    if (cached && (cached.url || now() - cached.checkedAt < missRetryMs)) {
      return Promise.resolve(cached.url);
    }
    const inFlight = pending.get(key);
    if (inFlight) return inFlight;
    const request = Promise.resolve().then(() => fetchIcon(key)).then(url => {
      resolved.set(key, { url, checkedAt: now() });
      return url;
    }).finally(() => pending.delete(key));
    pending.set(key, request);
    return request;
  };
};
