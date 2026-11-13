import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/actionSearch.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { matchesQuery, groupOf, nameOf } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

const stream = { id: "obs.start_stream", actionLabel: "OBS: Toggle stream", category: "OBS", description: "Go live" };
const cpu = { id: "sys.cpu", actionLabel: "System: CPU use (live)", description: "Shows how busy the processor is" };

test("every typed word must match, in any order and case", () => {
  assert.equal(matchesQuery(stream, ""), true);
  assert.equal(matchesQuery(stream, "stream obs"), true);
  assert.equal(matchesQuery(stream, "STREAM"), true);
  assert.equal(matchesQuery(stream, "stream record"), false);
  assert.equal(matchesQuery(cpu, "processor"), true);
  assert.equal(matchesQuery(cpu, "system"), true);
});

test("groups come from the category, else the label prefix", () => {
  assert.equal(groupOf(stream), "OBS");
  assert.equal(groupOf(cpu), "System");
  assert.equal(groupOf({ id: "x", actionLabel: "Plain" }), "Other");
  assert.equal(nameOf(stream), "Toggle stream");
});
