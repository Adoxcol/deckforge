import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/audioOutputs.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { outputParamOptions } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("output choices offer every output, each pair and each single output", () => {
  const options = outputParamOptions([
    { id: "1", name: "Speakers", isDefault: true },
    { id: "2", name: "Headphones", isDefault: false },
    { id: "3", name: "TV", isDefault: false },
  ]);
  assert.equal(options[0].value, "");
  assert.deepEqual(options.slice(1, 4).map((o) => o.value), ["Speakers | Headphones", "Speakers | TV", "Headphones | TV"]);
  assert.equal(options.at(-1).label, "Always TV");
  assert.equal(options.length, 7);
});
