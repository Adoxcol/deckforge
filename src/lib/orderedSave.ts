/** Run writes in request order, including after an earlier write fails. */
export function createOrderedSave<T, R>(persist: (value: T) => Promise<R>) {
  let tail: Promise<unknown> = Promise.resolve();
  return (value: T): Promise<R> => {
    const pending = tail.then(() => persist(value));
    tail = pending.catch(() => {});
    return pending;
  };
}
