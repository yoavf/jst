import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { demoFileAt, demoRedirect } from "../functions/demo.gif.js";

test("the demo endpoint can select every recorded GIF", () => {
  assert.equal(demoFileAt(0), "changed-today.gif");
  assert.equal(demoFileAt(1), "clear-port-8080.gif");
  assert.equal(demoFileAt(2), "largest-files.gif");
  assert.equal(demoFileAt(3), "remove-ds-store.gif");
  assert.equal(demoFileAt(4), "zip-folder.gif");
});

test("the demo endpoint redirects without allowing the selection to be cached", () => {
  const response = demoRedirect(2);

  assert.equal(response.status, 302);
  assert.equal(response.headers.get("Location"), "/demos/largest-files.gif");
  assert.equal(response.headers.get("Cache-Control"), "no-store");
});

test("only the random demo route invokes a Pages Function", async () => {
  const routes = JSON.parse(
    await readFile(new URL("../docs/_routes.json", import.meta.url), "utf8"),
  );

  assert.deepEqual(routes, {
    version: 1,
    include: ["/demo.gif"],
    exclude: [],
  });
});
