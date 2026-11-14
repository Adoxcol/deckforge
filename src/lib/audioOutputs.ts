import type { AudioOutput } from "../types/deckforge";

/** One choice per sound output, for the two "switch between" pickers. */
export function outputParamOptions(outputs: AudioOutput[]): { value: string; label: string }[] {
  return [...new Set(outputs.map((o) => o.name))].map((name) => ({ value: name, label: name }));
}

/** The two outputs a key flips between, from its stored "A | B". */
export function splitOutputPair(param: string): [string, string] {
  const [a = "", b = ""] = param.split("|").map((s) => s.trim());
  return [a, b];
}

export function joinOutputPair(a: string, b: string): string {
  return [a, b].filter(Boolean).join(" | ");
}
