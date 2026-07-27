import assert from "node:assert/strict";
import test from "node:test";
import { File } from "@bjorn3/browser_wasi_shim";
import {
  BoundedOpenFile,
  OutputBudget,
  OutputLimitError,
} from "./demo-output.js";

test("rejects bytes before a terminal-output buffer exceeds its budget", () => {
  const budget = new OutputBudget(5);

  budget.consume(3);
  assert.throws(() => budget.consume(3), OutputLimitError);
  assert.equal(budget.usedBytes, 3);
});

test("bounds non-terminal output before OpenFile allocates it", () => {
  const budget = new OutputBudget(5);
  const file = new File(new Uint8Array());
  const output = new BoundedOpenFile(file, budget);

  output.fd_write(new Uint8Array([1, 2, 3]));
  assert.throws(
    () => output.fd_write(new Uint8Array([4, 5, 6])),
    OutputLimitError,
  );
  assert.equal(file.data.byteLength, 3);
});
