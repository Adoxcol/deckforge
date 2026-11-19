import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/lib/profileHistory.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } });
const history = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const { initialClock, isClean, captureLibrary, pushEntry, beginEdit, noteSaved, noteRestored } = history;

test("undo back to the saved revision is clean and redo leaves it dirty", () => {
  let clock = initialClock();
  const original = [{ id: "a", name: "Original" }];
  const edited = beginEdit(clock);
  clock = edited.clock;
  const undo = [{ revision: edited.undoRevision, state: captureLibrary(original) }];
  assert.equal(isClean(clock), false);
  const current = [{ id: "a", name: "Edited" }];
  const redo = [{ revision: clock.current, state: captureLibrary(current) }];
  clock = noteRestored(clock, undo[0].revision);
  assert.equal(isClean(clock), true);
  clock = noteRestored(clock, redo[0].revision);
  assert.equal(isClean(clock), false);
});

test("a save finishes at the revision it started, so a newer edit stays dirty", () => {
  let clock = beginEdit(initialClock()).clock;
  const startedAt = clock.current;
  clock = beginEdit(clock).clock;
  clock = noteSaved(clock, startedAt);
  assert.equal(isClean(clock), false);
  clock = noteSaved(clock, clock.current);
  assert.equal(isClean(clock), true);
});

test("history keeps the same profile objects and drops entries past the cap", () => {
  const first = { id: "a" };
  const second = { id: "b" };
  const captured = captureLibrary([first, second]);
  assert.equal(captured[0], first);
  assert.equal(captured[1], second);
  assert.notEqual(captured, [first, second]);
  let stack = [];
  for (let revision = 0; revision < 25; revision++) stack = pushEntry(stack, { revision, state: revision });
  assert.equal(stack.length, 20);
  assert.equal(stack[0].revision, 5);
  assert.equal(stack[19].revision, 24);
});
