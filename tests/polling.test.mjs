import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/polling.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { startPolling } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("slow telemetry never overlaps and stopping drops an in-flight result", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  let reads = 0;
  let resolve;
  const published = [];
  const stop = startPolling(() => { reads++; return new Promise(r => { resolve = r; }); }, value => published.push(value), 5000);
  t.mock.timers.tick(5000);
  assert.equal(reads, 1);
  t.mock.timers.tick(60000);
  assert.equal(reads, 1);
  resolve(42);
  await Promise.resolve();
  assert.deepEqual(published, [42]);
  t.mock.timers.tick(4999);
  assert.equal(reads, 1);
  t.mock.timers.tick(1);
  assert.equal(reads, 2);
  stop();
  resolve(99);
  await Promise.resolve();
  t.mock.timers.tick(60000);
  assert.equal(reads, 2);
  assert.deepEqual(published, [42]);
});

test("failed telemetry retries and stopping cancels the scheduled read", async t => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  let reads = 0;
  const published = [];
  const stop = startPolling(async () => { if (++reads === 1) throw new Error("unavailable"); return 7; }, value => published.push(value), 5000);
  t.mock.timers.tick(5000);
  await Promise.resolve();
  assert.deepEqual(published, []);
  t.mock.timers.tick(5000);
  await Promise.resolve();
  assert.deepEqual(published, [7]);
  stop();
  t.mock.timers.tick(60000);
  assert.equal(reads, 2);
});
