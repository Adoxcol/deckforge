import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/lib/appAudio.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const { appAudioLook, encoderAudioLook } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const app = { exe: "spotify.exe", sessions: 2, minVolume: 20, maxVolume: 20, muted: false, mixedMute: false };
const face = { volumePercent: 99, appAudio: { "spotify.exe": app } };
test("per-app knobs never display master volume and handle distinct targets", () => {
  const knob = { cwAction: "audio.app_volume_up", cwParam: "Spotify.EXE", ccwAction: "audio.app_volume_down", ccwParam: "spotify.exe", pushAction: "audio.app_mute", pushParam: "spotify.exe" };
  assert.deepEqual(encoderAudioLook(knob, face), { title: "20%", level: 20, muted: false });
  assert.equal(encoderAudioLook({ ...knob, pushParam: "discord.exe" }, face).title, "Multiple apps");
  assert.equal(encoderAudioLook({ cwAction: "audio.volume_up" }, face).level, 99);
});
test("mixed sessions show a range instead of a misleading meter", () => {
  const mixed = { ...face, appAudio: { "spotify.exe": { ...app, maxVolume: 80, mixedMute: true } } };
  assert.deepEqual(appAudioLook("spotify.exe", mixed), { title: "20-80% MIX", level: null, muted: false });
  mixed.appAudio["spotify.exe"].muted = true;
  assert.deepEqual(appAudioLook("spotify.exe", mixed), { title: "Muted", level: 0, muted: true });
});
test("empty targets, unavailable sampling and missing sessions are distinct", () => {
  assert.equal(appAudioLook("", face).title, "Choose app");
  assert.equal(appAudioLook("spotify.exe", null).title, "Unavailable");
  assert.equal(appAudioLook("spotify.exe", { appAudio: {} }).title, "No session");
  assert.equal(appAudioLook("spotify.exe", { appAudio: null }).level, null);
});

test("knobs mixing app and master audio do not present a single app meter", () => {
  for (const master of ["audio.volume_up", "audio.volume_down", "audio.volume_mute"]) {
    for (const mixed of [
      { cwAction: "audio.app_volume_up", cwParam: "spotify.exe", ccwAction: master },
      { cwAction: master, pushAction: "audio.app_mute", pushParam: "spotify.exe" },
      { ccwAction: "audio.app_volume_down", ccwParam: "spotify.exe", pushAction: master },
    ]) {
      assert.deepEqual(encoderAudioLook(mixed, face), { title: "Multiple targets", level: null, muted: false });
      assert.deepEqual(encoderAudioLook(mixed, null), { title: "Multiple targets", level: null, muted: false });
    }
  }
});
