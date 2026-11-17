import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/appRuleConflicts.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { assignUnrestrictedApp, conflictingRule } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const rule = (id, name, profile, priority, title = "") => ({ id, processName:name, targetProfileId:profile, targetPageId:"first", windowTitleContains:title, priority, enabled:true });
test("adding Spotify elsewhere moves existing unrestricted rules and resolves duplicates", () => {
  const rules = [rule("a", "Spotify.exe", "streaming", 4), rule("b", "spotify", "gaming", 5), rule("c", "spotify.exe", "work", 2, "Special")];
  assert.equal(conflictingRule(rules, rules[1]).id, "a");
  const updated = assignUnrestrictedApp(rules, "SPOTIFY.EXE", "work", "music");
  assert.equal(updated.length, 3);
  assert.deepEqual(updated.filter(r => !r.windowTitleContains).map(r => [r.id,r.targetProfileId,r.targetPageId,r.enabled]), [["a","work","music",true],["b","work","music",false]]);
  assert.deepEqual(updated[2], rules[2]);
  assert.equal(conflictingRule(updated, updated[0]).id, "a");
  assert.equal(conflictingRule(updated, updated[1]), null);
  assert.equal(rules[0].targetProfileId, "streaming");
});
test("new app creates one enabled rule", () => {
  const updated = assignUnrestrictedApp([rule("a", "Spotify.exe", "streaming", 4)], "Discord", "gaming", "first");
  assert.equal(updated.length, 2);
  assert.equal(updated[1].processName, "Discord");
  assert.equal(updated[1].priority, 5);
});
