import type { AudioOutput } from "../types/deckforge";

/** Choices for the "Switch sound output" key: every output, one output, or a pair to flip between. */
export function outputParamOptions(outputs: AudioOutput[]): { value: string; label: string }[] {
  const names = [...new Set(outputs.map((o) => o.name))];
  const options = [{ value: "", label: "Every output, one after another" }];
  for (let i = 0; i < names.length; i++) {
    for (let j = i + 1; j < names.length; j++) {
      options.push({ value: `${names[i]} | ${names[j]}`, label: `${names[i]} ⇄ ${names[j]}` });
    }
  }
  for (const name of names) options.push({ value: name, label: `Always ${name}` });
  return options;
}
