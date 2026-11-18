import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/appSwitchPreview.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { previewAppSwitch } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const profiles = [
  { id: "stream", name: "Streaming", pages: [{ id: "main", name: "Main" }, { id: "music", name: "Music" }] },
  { id: "gaming", name: "Gaming", pages: [{ id: "game", name: "Game" }, { id: "audio", name: "Audio" }] },
  { id: "work", name: "Work", pages: [{ id: "desk", name: "Desk" }] },
];
const rule = (id, processName, targetProfileId, targetPageId, priority = 1, windowTitleContains = "") =>
  ({ id, processName, targetProfileId, targetPageId, priority, windowTitleContains, enabled: true });

test("Spotify page rules in each profile and a global profile rule resolve in one step", () => {
  const rules = [rule("global", "spotify.exe", "gaming", ""), rule("stream-page", "spotify.exe", "stream", "music"), rule("game-page", "spotify.exe", "gaming", "audio")];
  assert.deepEqual(["stream", "gaming", "work"].map(id => {
    const result = previewAppSwitch(rules, profiles, id, 0, "SPOTIFY.EXE", "Spotify", false);
    return [result.profileId, result.pageIndex, result.ruleId, result.pageRuleId];
  }), [["stream", 1, "stream-page", null], ["gaming", 1, "game-page", null], ["gaming", 1, "global", "game-page"]]);
});

test("title filters, disabled rules, deleted pages and any-game priority are explained", () => {
  const rules = [rule("game", "@game", "gaming", "", 1), rule("specific", "valorant.exe", "stream", "", 9)];
  assert.equal(previewAppSwitch(rules, profiles, "work", 0, "valorant.exe", "", true).ruleId, "specific");
  assert.equal(previewAppSwitch(rules, profiles, "work", 0, "other.exe", "", true).ruleId, "game");
  const title = rule("title", "spotify.exe", "stream", "music", 1, "Player");
  assert.equal(previewAppSwitch([title], profiles, "stream", 0, "spotify.exe", "Home", false).ruleId, null);
  assert.equal(previewAppSwitch([title], profiles, "stream", 0, "spotify.exe", "Music Player", false).pageIndex, 1);
  assert.equal(previewAppSwitch([{ ...title, enabled: false }], profiles, "stream", 0, "spotify.exe", "Music Player", false).ruleId, null);
  assert.match(previewAppSwitch([rule("gone", "spotify.exe", "stream", "deleted")], profiles, "stream", 0, "spotify.exe", "", false).reason, /deleted page/);
});
