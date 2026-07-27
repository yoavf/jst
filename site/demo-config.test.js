import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import viteConfig from "../vite.config.js";

const [pageScript, pageStyles] = await Promise.all([
  readFile(new URL("../docs/script.js", import.meta.url), "utf8"),
  readFile(new URL("../docs/styles.css", import.meta.url), "utf8"),
]);

test("local development proxies the playground through the hardened demo endpoint", () => {
  const proxy = viteConfig.server.proxy["/api/jst-demo"];

  assert.equal(proxy.rewrite(), "/demo");
  assert.equal(proxy.headers.origin, "https://jst.sh");
});

test("starts the worker sandbox without requiring cross-origin isolation", () => {
  assert.doesNotMatch(pageScript, /if \(!window\.crossOriginIsolated\)/);
});

test("hides the example switcher whenever the demo dialog is open", () => {
  assert.match(
    pageStyles,
    /\.demo-dialog \.another-example\s*{\s*display: none;\s*}/,
  );
});
