import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/shareFiles.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { placeBackup } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);

const profiles = [
  { id: "a", name: "Streaming", pages: [{ id: "page1", name: "Home", stripMedia: { mediaId: "restored-media" },
    keys: [{ actionId: "profile.switch", actionParam: "b", media: { mediaId: "restored-media", speed: 1.5 } }],
    touchZones: [{ tapActionId: "macro.run", tapActionParam: "My macro" }],
    encoders: [{ cwAction: "profile.switch", cwParam: "Gaming", ccwAction: "macro.run", ccwParam: "m", pushAction: "profile.switch", pushParam: "a" }] }] },
  { id: "b", name: "Gaming", pages: [] },
];
const macros = [{ id: "m", name: "My macro", steps: [{ stepType: "SwitchProfile", parameter: "Streaming" }, { stepType: "Shortcut", parameter: "Ctrl+C" }] }];

test("backup copies remap cross-profile and macro references across every control", () => {
  const before = structuredClone({ profiles, macros });
  const placed = placeBackup(profiles, macros, profiles, macros);
  const page = placed.profiles[0].pages[0];
  assert.equal(page.keys[0].actionParam, placed.profiles[1].id);
  assert.equal(page.touchZones[0].tapActionParam, placed.macros[0].id);
  assert.equal(page.encoders[0].cwParam, placed.profiles[1].id);
  assert.equal(page.encoders[0].ccwParam, placed.macros[0].id);
  assert.equal(page.encoders[0].pushParam, placed.profiles[0].id);
  assert.equal(placed.macros[0].steps[0].parameter, placed.profiles[0].id);
  assert.equal(placed.macros[0].steps[1].parameter, "Ctrl+C");
  assert.deepEqual(page.keys[0].media, profiles[0].pages[0].keys[0].media);
  assert.deepEqual(page.stripMedia, profiles[0].pages[0].stripMedia);
  assert.deepEqual({ profiles, macros }, before);
});

test("repeated restores allocate unique IDs and names without replacing existing data", () => {
  const first = placeBackup(profiles, macros, profiles, macros);
  const existing = [...profiles, ...first.profiles];
  const second = placeBackup(profiles, macros, existing, [...macros, ...first.macros]);
  assert.equal(new Set([...existing, ...second.profiles].map(p => p.id)).size, 6);
  assert.equal(new Set([...existing, ...second.profiles].map(p => p.name)).size, 6);
  assert.notEqual(first.macros[0].id, second.macros[0].id);
});
