import test from "node:test";
import assert from "node:assert/strict";
import * as original from "simple-icons";
import { brandIconsPlugin } from "../build/brandIcons.mjs";

test("compact logo module preserves every searchable title, color and drawable path", async () => {
  const plugin = brandIconsPlugin();
  assert.equal(plugin.resolveId("unrelated-module"), null);
  assert.equal(await plugin.load("unrelated-module"), null);
  const source = await plugin.load(plugin.resolveId("virtual:brand-icons"));
  const { default: icons } = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
  const expected = Object.values(original);
  assert.equal(icons.length, expected.length);
  for (let i = 0; i < expected.length; i++) {
    const { title, slug, hex, path } = expected[i];
    assert.deepEqual(icons[i], { title, slug, hex, path });
  }
});

test("app marks load only their small fallback catalog", async () => {
  const plugin = brandIconsPlugin();
  const source = await plugin.load(plugin.resolveId("virtual:app-brand-icons"));
  const { default: icons } = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
  assert.ok(Buffer.byteLength(source) < 50_000, "app marks must not load the full logo catalog");
  assert.equal(Object.keys(icons).length, 16);
  assert.deepEqual(icons.steamwebhelper, icons.steam);
  assert.deepEqual(icons["docker desktop"], icons.docker);
  const chrome = Object.values(original).find((icon) => icon.slug === "googlechrome");
  assert.deepEqual(icons.chrome, { hex: chrome.hex, path: chrome.path });
  assert.ok(Object.values(icons).every((icon) => icon?.hex && icon?.path));
});
