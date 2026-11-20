/** `vol:80|C:\sounds\horn.wav`. A bare path means full volume. */
export function splitSound(param: string): { volume: number; path: string } {
  const match = /^vol:(\d{1,3})\|([\s\S]*)$/.exec(param);
  if (!match) return { volume: 100, path: param };
  const volume = Math.min(100, Math.max(0, Number(match[1])));
  return { volume, path: match[2] };
}

export function joinSound(volume: number, path: string): string {
  const clamped = Math.max(0, Math.min(100, Math.round(volume)));
  return `vol:${clamped}|${path}`;
}
