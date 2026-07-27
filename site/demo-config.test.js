import assert from "node:assert/strict";
import test from "node:test";
import viteConfig from "../vite.config.js";

test("local development proxies the playground through the hardened demo endpoint", () => {
  const proxy = viteConfig.server.proxy["/api/jst-demo"];

  assert.equal(proxy.rewrite(), "/demo");
  assert.equal(proxy.headers.origin, "https://jst.sh");
});
