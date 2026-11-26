import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/orderedSave.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
});
const { createOrderedSave } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("Game Mode config writes keep their order and continue after a failure", async () => {
  const writes = [];
  let releaseFirst;
  const save = createOrderedSave(async value => {
    writes.push(value);
    if (value === "first") await new Promise(resolve => { releaseFirst = resolve; });
    if (value === "second") throw new Error("disk full");
    return value;
  });
  const first = save("first");
  const second = save("second");
  const third = save("third");
  await Promise.resolve();
  assert.deepEqual(writes, ["first"]);
  releaseFirst();
  assert.equal(await first, "first");
  await assert.rejects(second, /disk full/);
  assert.equal(await third, "third");
  assert.deepEqual(writes, ["first", "second", "third"]);
});
