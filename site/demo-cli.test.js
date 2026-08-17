import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { JST_HELP, JST_VERSION, parseJstInvocation } from "./demo-cli.js";

test("supports the complete JST option surface", () => {
  assert.match(JST_HELP, /--yolo/);
  assert.match(JST_HELP, /--interactive/);
  assert.match(JST_HELP, /--dry/);
  assert.match(JST_HELP, /--status/);
  assert.equal(JST_VERSION, "0.4.0");

  assert.deepEqual(parseJstInvocation(""), { action: "help" });
  assert.deepEqual(parseJstInvocation("--help"), { action: "help" });
  assert.deepEqual(parseJstInvocation("-h"), { action: "help" });
  assert.deepEqual(parseJstInvocation("--version"), { action: "version" });
  assert.deepEqual(parseJstInvocation("-V"), { action: "version" });
  assert.deepEqual(parseJstInvocation("--status"), { action: "status" });
  assert.deepEqual(parseJstInvocation("--dry find hidden files"), {
    action: "translate",
    dry: true,
    input: "find hidden files",
    interactive: false,
    status: false,
    yolo: false,
  });
  assert.deepEqual(parseJstInvocation("-i 'create a photos folder'"), {
    action: "translate",
    dry: false,
    input: "create a photos folder",
    interactive: true,
    status: false,
    yolo: false,
  });
  assert.deepEqual(parseJstInvocation("--yolo list files"), {
    action: "translate",
    dry: false,
    input: "list files",
    interactive: false,
    status: false,
    yolo: true,
  });
});

test("rejects invalid JST option combinations locally", () => {
  assert.match(parseJstInvocation("--wat list files").error, /unexpected argument/);
  assert.match(parseJstInvocation("--dry").error, /prompt is required/);
  assert.match(parseJstInvocation("--dry -i list files").error, /cannot be used/);
  assert.match(parseJstInvocation("--yolo --dry list files").error, /cannot be used/);
  assert.match(parseJstInvocation("--status list files").error, /cannot be combined/);
});

test("keeps the browser version aligned with the CLI crate", async () => {
  const cargoToml = await readFile(
    new URL("../crates/cli/Cargo.toml", import.meta.url),
    "utf8",
  );
  assert.equal(cargoToml.match(/^version = "([^"]+)"/m)?.[1], JST_VERSION);
});
