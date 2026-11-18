import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/appIconCache.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { createAppIconLoader } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

test("concurrent tiles for one app share one backend lookup", async () => {
  let calls = 0;
  const load = createAppIconLoader(async () => { calls++; return "data:image/png;base64,a"; });
  const icons = await Promise.all(Array.from({ length: 12 }, () => load(" SPOTIFY.EXE ")));
  assert.equal(calls, 1); // Without coalescing, 12 tiles make 12 process/filesystem scans.
  assert.equal(new Set(icons).size, 1);
  await load("spotify.exe");
  assert.equal(calls, 1);
});

test("missing icons are cached briefly then retried; errors are not cached", async () => {
  let time = 0;
  let calls = 0;
  const load = createAppIconLoader(async () => { calls++; return null; }, () => time, 100);
  await Promise.all(Array.from({ length: 12 }, () => load("unknown.exe")));
  assert.equal(calls, 1);
  time = 99;
  await load("unknown.exe");
  assert.equal(calls, 1);
  time = 100;
  await load("unknown.exe");
  assert.equal(calls, 2);

  let attempts = 0;
  const retry = createAppIconLoader(async () => { if (++attempts === 1) throw Error("temporary"); return "ok"; });
  await assert.rejects(retry("retry.exe"));
  assert.equal(await retry("retry.exe"), "ok");
});
