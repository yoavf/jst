import assert from "node:assert/strict";
import test from "node:test";
import { createReminderCopyState } from "../docs/reminder-copy-state.js";

function createTimer() {
  const callbacks = new Map();
  let nextId = 0;

  return {
    clearTimeout(id) {
      callbacks.delete(id);
    },
    run(id) {
      callbacks.get(id)?.();
    },
    setTimeout(callback) {
      const id = nextId;
      nextId += 1;
      callbacks.set(id, callback);
      return id;
    },
  };
}

test("keeps the latest reminder copy result when clipboard writes settle out of order", () => {
  const states = [];
  const timer = createTimer();
  const copyState = createReminderCopyState({
    clearTimeout: timer.clearTimeout,
    setTimeout: timer.setTimeout,
    setState: (state) => states.push(state),
  });

  const firstCopy = copyState.begin();
  const secondCopy = copyState.begin();

  copyState.succeed(secondCopy);
  copyState.fail(firstCopy);

  assert.deepEqual(states, ["copied"]);
});

test("ignores a stale copy success after a later copy succeeds", () => {
  const states = [];
  const timer = createTimer();
  const copyState = createReminderCopyState({
    clearTimeout: timer.clearTimeout,
    setTimeout: timer.setTimeout,
    setState: (state) => states.push(state),
  });

  const firstCopy = copyState.begin();
  const secondCopy = copyState.begin();
  copyState.succeed(secondCopy);
  copyState.succeed(firstCopy);

  timer.run(0);

  assert.deepEqual(states, ["copied", "copy"]);
});
