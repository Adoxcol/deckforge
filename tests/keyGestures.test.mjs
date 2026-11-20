import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/shareFiles.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { parseProfileFile, macroIdsUsed } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

const file = JSON.stringify({
  kind: "deckforge.profile",
  version: 1,
  profile: {
    id: "stream",
    name: "Streaming",
    pages: [{
      id: "main",
      name: "Main",
      keys: [{
        index: 0,
        title: "Mic",
        actionId: "audio.mute_mic",
        doublePress: { actionId: "macro.run", actionLabel: "Macro: Scene", actionParam: "scene-macro" },
        hold: { actionId: "obs.toggle_record", actionLabel: "Record", actionParam: "" },
      }],
    }],
  },
});

test("a shared profile keeps a double-press and a hold", () => {
  const { profile } = parseProfileFile(file);
  const key = profile.pages[0].keys[0];
  assert.equal(key.actionId, "audio.mute_mic");
  assert.equal(key.doublePress.actionId, "macro.run");
  assert.equal(key.doublePress.actionParam, "scene-macro");
  assert.equal(key.hold.actionId, "obs.toggle_record");
  assert.equal(key.hold.actionParam, "");
});

test("a hold that runs a macro is packed with that macro", () => {
  const { profile } = parseProfileFile(file);
  assert.deepEqual(macroIdsUsed(profile), ["scene-macro"]);
});

test("an older profile file without gestures still loads", () => {
  const older = JSON.stringify({
    kind: "deckforge.profile",
    version: 1,
    profile: { id: "old", name: "Old", pages: [{ id: "p", name: "Page", keys: [{ index: 1, title: "Go", actionId: "media.play_pause" }] }] },
  });
  const key = parseProfileFile(older).profile.pages[0].keys[0];
  assert.equal(key.doublePress, undefined);
  assert.equal(key.hold, undefined);
  assert.equal(key.latch, undefined);
});

test("stay lit survives a shared profile", () => {
  const text = JSON.stringify({
    kind: "deckforge.profile",
    version: 1,
    profile: { id: "p", name: "P", pages: [{ id: "main", name: "Main", keys: [{ index: 0, title: "Cam", actionId: "custom.hotkey", actionParam: "Ctrl+1", latch: true }] }] },
  });
  assert.equal(parseProfileFile(text).profile.pages[0].keys[0].latch, true);
});
