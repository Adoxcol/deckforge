/** Schedule the next read only after the previous one settles. */
export function startPolling<T>(read: () => Promise<T>, publish: (value: T) => void, delayMs: number) {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout>;
  const poll = async () => {
    try {
      const value = await read();
      if (!stopped) publish(value);
    } catch {
      // Retain the last successful value; retry after the normal interval.
    } finally {
      if (!stopped) timer = setTimeout(poll, delayMs);
    }
  };
  timer = setTimeout(poll, delayMs);
  return () => {
    stopped = true;
    clearTimeout(timer);
  };
}
