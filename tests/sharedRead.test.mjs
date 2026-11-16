import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/sharedRead.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { createSharedRead } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("three app pickers share one scan and expiry permits fresh readings", async () => {
  let time = 0, reads = 0, complete;
  const read = createSharedRead(() => { reads++; return new Promise(resolve => { complete = resolve; }); }, 500, () => time);
  const first = read(), second = read(), third = read(true);
  assert.equal(first, second);
  assert.equal(first, third);
  await Promise.resolve();
  assert.equal(reads, 1);
  time = 1000;
  complete(["spotify.exe"]);
  assert.deepEqual(await Promise.all([first, second, third]), [["spotify.exe"], ["spotify.exe"], ["spotify.exe"]]);
  time = 1499;
  assert.deepEqual(await read(), ["spotify.exe"]);
  assert.equal(reads, 1);
  time = 1500;
  const fresh = read();
  await Promise.resolve();
  assert.equal(reads, 2);
  complete([]);
  assert.deepEqual(await fresh, []);
});

test("manual refresh bypasses cache and failures cannot leave stale successes", async () => {
  let reads = 0;
  const read = createSharedRead(async () => { if (++reads === 2) throw new Error("scan failed"); return reads; }, 500, () => 0);
  assert.equal(await read(), 1);
  await assert.rejects(read(true), /scan failed/);
  assert.equal(await read(), 3);
  assert.equal(await read(), 3);
  assert.equal(await read(true), 4);
});
