import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/mediaLibrary.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { parseLibraryPrefs, libraryItems } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
test("library preferences tolerate corrupt storage and sanitize IDs", () => {
  for (const value of [null, "oops", "null"]) assert.deepEqual(parseLibraryPrefs(value), { favorites: [], recent: [] });
  assert.deepEqual(parseLibraryPrefs('{"favorites":["a",3,"a","b"],"recent":["b","a","b"]}'), { favorites: ["a", "b"], recent: ["b", "a"] });
});
test("media search and favorite/recent views retain ordering and skip missing media", () => {
  const clips = [{ id: "a", fileName: "Neon pulse" }, { id: "b", fileName: "Blue orbit" }];
  const prefs = { favorites: ["a", "missing"], recent: ["missing", "b", "a"] };
  assert.deepEqual(libraryItems(clips, "PULSE neon", "My media", prefs).map(c => c.id), ["a"]);
  assert.deepEqual(libraryItems(clips, "", "Favorites", prefs).map(c => c.id), ["a"]);
  assert.deepEqual(libraryItems(clips, "", "Recent", prefs).map(c => c.id), ["b", "a"]);
  assert.deepEqual(libraryItems(clips, "other", "Recent", prefs), []);
  assert.equal(clips[0].id, "a");
});
