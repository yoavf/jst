import assert from "node:assert/strict";
import test from "node:test";
import { demoFileAt, demoRedirect } from "../workers/demo-gif.js";

test("the demo endpoint can select every recorded GIF", () => {
  assert.equal(demoFileAt(0), "changed-today.gif");
  assert.equal(demoFileAt(1), "clear-port-8080.gif");
  assert.equal(demoFileAt(2), "remove-ds-store.gif");
  assert.equal(demoFileAt(3), "zip-folder.gif");
});

test("the demo endpoint redirects without allowing the selection to be cached", () => {
  const response = demoRedirect(3);

  assert.equal(response.status, 302);
  assert.equal(response.headers.get("Location"), "https://jst.sh/demos/zip-folder.gif");
  assert.equal(response.headers.get("Cache-Control"), "no-store");
});
