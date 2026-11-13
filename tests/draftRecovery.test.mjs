import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/draftRecovery.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { createDraftRecovery } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; };

test("drafts snapshot input and coalesce rapid edits", async () => {
  const writes = [];
  const queue = createDraftRecovery(async v => writes.push(v), () => {}, 60000);
  queue.schedule({ title: "old" });
  const latest = { title: "latest" }; queue.schedule(latest); latest.title = "mutated";
  assert.equal(await queue.flush(), true);
  assert.deepEqual(writes, [{ title: "latest" }]);
});

test("clearing waits for an in-flight write and cannot resurrect its draft", async () => {
  const gate = deferred(); const writes = [];
  const queue = createDraftRecovery(async v => { if (v) await gate.promise; writes.push(v); }, () => {}, 60000);
  queue.schedule({ title: "old" }); const flushing = queue.flush();
  await Promise.resolve();
  queue.schedule({ title: "new" });
  queue.pause();
  const clearing = queue.clear();
  queue.schedule({ title: "ignored during quit" });
  gate.resolve();
  await flushing; assert.equal(await clearing, true);
  assert.deepEqual(writes, [{ title: "old" }, null]);
});

test("draft failures retain the latest snapshot for retry and report failed clears", async () => {
  let fail = true; const writes = []; const statuses = [];
  const queue = createDraftRecovery(async v => { if (fail) throw new Error("disk full"); writes.push(v); }, s => statuses.push(s), 60000);
  queue.schedule({ title: "recover me" }); assert.equal(await queue.flush(), false);
  assert.equal(statuses.at(-1).error, "disk full");
  fail = false; queue.retry(); assert.equal(await queue.flush(), true);
  assert.deepEqual(writes, [{ title: "recover me" }]);
  fail = true; assert.equal(await queue.clear(), false);
  assert.equal(statuses.at(-1).pending, true);
  fail = false; queue.retry(); await queue.flush(); assert.equal(writes.at(-1), null);
});

test("an old failed write cannot replace the newest recovery status", async () => {
  const gate = deferred(); const statuses = [];
  const queue = createDraftRecovery(async v => { if (v.title === "old") { await gate.promise; throw new Error("old failure"); } }, s => statuses.push(s), 60000);
  queue.schedule({ title: "old" }); const first = queue.flush(); await Promise.resolve();
  queue.schedule({ title: "new" }); const second = queue.flush(); gate.resolve();
  await first; assert.equal(await second, true);
  assert.deepEqual(statuses.at(-1), { pending: false, error: null });
  assert.ok(!statuses.some(s => s.error === "old failure"));
});

test("cancelled quit resumes recovery for retained edits", async () => {
  const writes = [];
  const queue = createDraftRecovery(async v => writes.push(v), () => {}, 60000);
  queue.pause(); await queue.clear(); queue.resume(); queue.schedule({ title: "still editing" }); await queue.flush();
  assert.deepEqual(writes, [null, { title: "still editing" }]);
});
