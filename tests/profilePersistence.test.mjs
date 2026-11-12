import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/profilePersistence.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { createProfilePersistence, profileSnapshot } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; };

test("undo to saved revision is clean and redo away from it is dirty", async () => {
  const initial = [{ name: "Original", pages: [{ name: "Music" }] }];
  const edited = [{ name: "Edited", pages: [{ name: "Renamed" }] }];
  let saved = profileSnapshot(initial);
  const writer = createProfilePersistence(async () => {}, revision => { saved = revision; });
  assert.notEqual(profileSnapshot(edited), saved);
  assert.equal(profileSnapshot(structuredClone(initial)), saved);
  await writer.save(edited);
  assert.notEqual(profileSnapshot(initial), saved);
  assert.equal(profileSnapshot(structuredClone(edited)), saved);
});

test("edits during an in-flight save remain dirty and input is snapshotted", async () => {
  const gate = deferred(); const input = [{ name: "Saved version" }]; let saved; let written;
  const writer = createProfilePersistence(async value => { written = value; await gate.promise; }, revision => { saved = revision; });
  const saving = writer.save(input);
  input[0].name = "Newer edit";
  await Promise.resolve();
  assert.equal(written[0].name, "Saved version");
  assert.equal(writer.isPending(), true);
  gate.resolve(); await saving; await writer.flush();
  assert.notEqual(profileSnapshot(input), saved);
  assert.equal(writer.savedSnapshot(), saved);
  assert.equal(writer.isPending(), false);
});

test("writes serialize and failed saves do not advance the saved revision or block retry", async () => {
  const gate = deferred(); const writes = []; let saved = "original";
  const writer = createProfilePersistence(async value => {
    writes.push(value); if (value === "first") { await gate.promise; throw new Error("disk full"); }
  }, revision => { saved = revision; });
  const first = writer.save("first"); const rejected = assert.rejects(first, /disk full/);
  const second = writer.save("second");
  await Promise.resolve(); assert.deepEqual(writes, ["first"]); assert.equal(saved, "original");
  gate.resolve(); await rejected; await second; await writer.flush();
  assert.deepEqual(writes, ["first", "second"]); assert.equal(saved, profileSnapshot("second"));
});

test("quit flush waits for writes added while an earlier save is finishing", async () => {
  const first = deferred(), second = deferred();
  const writer = createProfilePersistence(value => value === 1 ? first.promise : second.promise, () => {});
  writer.save(1); let finished = false;
  const flushing = writer.flush().then(() => { finished = true; });
  writer.save(2); first.resolve();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(finished, false);
  second.resolve(); await flushing;
  assert.equal(writer.isPending(), false);
});
