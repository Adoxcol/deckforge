import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/bindingSearch.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { searchBindings } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

const profiles = [
  {
    id: "stream",
    name: "Streaming",
    pages: [
      {
        id: "main",
        name: "Main",
        keys: [
          { index: 2, title: "Mic", actionId: "audio.mute_mic", actionLabel: "Mute microphone", actionParam: "", doublePress: { actionId: "macro.run", actionLabel: "Scene", actionParam: "scene-1" } },
          { index: 3, title: "Play", actionId: "media.play_pause", actionLabel: "Play", actionParam: "" },
        ],
        touchZones: [{ index: 0, label: "Spotify", valueText: "Next", tapActionId: "media.next", tapActionParam: "" }],
        encoders: [{ index: 0, label: "Game", valueText: "", cwAction: "audio.app_volume_up", cwParam: "game.exe", ccwAction: "audio.app_volume_down", ccwParam: "game.exe", pushAction: "audio.app_mute", pushParam: "game.exe" }],
      },
      { id: "desk", name: "Desktop", keys: [{ index: 0, title: "Files", actionId: "sys.hotkey", actionLabel: "Explorer", actionParam: "Win+E" }], touchZones: [], encoders: [] },
    ],
  },
  {
    id: "edit",
    name: "Editing",
    pages: [{ id: "cut", name: "Cut", keys: [{ index: 1, title: "Mic", actionId: "custom.hotkey", actionLabel: "Mute", actionParam: "Ctrl+Shift+M" }], touchZones: [], encoders: [] }],
  },
];

test("every word must match and the hit names the profile, page, and control", () => {
  const hits = searchBindings(profiles, "mic mute", [{ id: "scene-1", name: "Scene change" }]);
  assert.equal(hits.length, 2);
  assert.equal(hits[0].profileName, "Streaming");
  assert.equal(hits[0].where, "Key 3 · double-press");
  assert.equal(hits[1].profileId, "edit");
  assert.equal(hits[1].pageName, "Cut");
});

test("a macro is found by its name, and a knob by the app it controls", () => {
  assert.equal(searchBindings(profiles, "scene change", [{ id: "scene-1", name: "Scene change" }])[0].index, 2);
  const knob = searchBindings(profiles, "game.exe")[0];
  assert.equal(knob.surface, "encoder");
  assert.equal(knob.where, "Knob 1");
});

test("an empty query lists nothing", () => {
  assert.deepEqual(searchBindings(profiles, "   "), []);
});
