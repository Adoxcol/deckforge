import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

// Exercise the production TypeScript utility without a browser or new test dependencies.
const source = await readFile(new URL("../src/lib/saveQueue.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
});
const { createSaveQueue } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("failed edits stay pending and retry writes the retained value", async () => {
  let fail = true;
  const writes = [], statuses = [];
  const queue = createSaveQueue(async value => {
    writes.push(value);
    if (fail) throw new Error("disk full");
  }, status => statuses.push(status));
  queue.save("my settings");
  assert.equal(await queue.flush(), false);
  assert.deepEqual(statuses.at(-1), { pending: true, saving: false, error: "disk full" });
  fail = false;
  queue.retry();
  assert.equal(await queue.flush(), true);
  assert.deepEqual(writes, ["my settings", "my settings"]);
  assert.deepEqual(statuses.at(-1), { pending: false, saving: false, error: null });
});

test("rapid edits serialize and skip superseded queued values", async () => {
  let release;
  const blocked = new Promise(resolve => { release = resolve; });
  const writes = [], statuses = [];
  const queue = createSaveQueue(async value => {
    writes.push(value);
    if (value === "first") await blocked;
  }, status => statuses.push(status));
  queue.save("first");
  await Promise.resolve();
  queue.save("second");
  queue.save("latest");
  assert.deepEqual(writes, ["first"]);
  release();
  assert.equal(await queue.flush(), true);
  assert.deepEqual(writes, ["first", "latest"]);
  assert.equal(statuses.filter(s => !s.pending).length, 1);
});

test("an older failure cannot replace the latest save status or block it", async () => {
  let reject;
  const blocked = new Promise((_, fail) => { reject = fail; });
  const statuses = [], writes = [];
  const queue = createSaveQueue(async value => {
    writes.push(value);
    if (value === "old") await blocked;
  }, status => statuses.push(status));
  queue.save("old");
  await Promise.resolve();
  queue.save("new");
  reject("old write failed");
  assert.equal(await queue.flush(), true);
  assert.deepEqual(writes, ["old", "new"]);
  assert.equal(statuses.at(-1).pending, false);
  assert.equal(statuses.some(s => s.error), false);
});
