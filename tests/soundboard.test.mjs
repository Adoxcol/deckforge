import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/soundboard.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { splitSound, joinSound } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("a bare path is full volume", () => {
  assert.deepEqual(splitSound("C:\\sounds\\horn.wav"), { volume: 100, path: "C:\\sounds\\horn.wav" });
});

test("volume and path round-trip", () => {
  const stored = joinSound(40, "C:\\sounds\\horn.wav");
  assert.equal(stored, "vol:40|C:\\sounds\\horn.wav");
  assert.deepEqual(splitSound(stored), { volume: 40, path: "C:\\sounds\\horn.wav" });
});

test("volume stays between 0 and 100", () => {
  assert.equal(joinSound(140, "a.wav"), "vol:100|a.wav");
  assert.equal(joinSound(-3, "a.wav"), "vol:0|a.wav");
  assert.equal(splitSound("vol:240|a.wav").volume, 100);
});
