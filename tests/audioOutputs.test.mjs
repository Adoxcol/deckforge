import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/audioOutputs.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { outputParamOptions, splitOutputPair, joinOutputPair } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("each output is one choice, and a pair round-trips", () => {
  const options = outputParamOptions([
    { id: "1", name: "Speakers", isDefault: true },
    { id: "2", name: "Headphones", isDefault: false },
  ]);
  assert.deepEqual(options.map((o) => o.value), ["Speakers", "Headphones"]);
  assert.deepEqual(splitOutputPair(joinOutputPair("Speakers", "Headphones")), ["Speakers", "Headphones"]);
  assert.deepEqual(splitOutputPair(""), ["", ""]);
  assert.equal(joinOutputPair("", "TV"), "TV");
});
