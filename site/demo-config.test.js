import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import viteConfig from "../vite.config.js";
import { statsTotalSizeStep } from "../docs/stats-display.js";

const [pageScript, pageStyles] = await Promise.all([
  readFile(new URL("../docs/script.js", import.meta.url), "utf8"),
  readFile(new URL("../docs/styles.css", import.meta.url), "utf8"),
]);
const pageMarkup = await readFile(
  new URL("../docs/index.html", import.meta.url),
  "utf8",
);

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

test("shrinks the stats tally after four digits with a readable floor", () => {
  assert.equal(statsTotalSizeStep("9,999"), 0);
  assert.equal(statsTotalSizeStep("10,000"), 1);
  assert.equal(statsTotalSizeStep("999,999"), 2);
  assert.equal(statsTotalSizeStep("1,000,000"), 3);
  assert.equal(statsTotalSizeStep("99,999,999"), 4);
  assert.equal(statsTotalSizeStep("1,000,000,000"), 5);
  assert.equal(statsTotalSizeStep("18,446,744,073,709,551,615"), 5);
});

test("ships the additional examples returned by the JST CLI", () => {
  assert.match(pageScript, /ps aux/);
  assert.match(pageScript, /--sort=-%mem/);
  assert.match(pageScript, /du -h --max-depth=1/);
  assert.match(pageScript, /for file in /);
  assert.match(pageScript, /screenshots\/\*\.png/);
  assert.match(pageScript, /openssl rand/);
  assert.match(pageScript, /cut -c1-32/);
});

test("leads with the most interesting examples and omits the expenses search", () => {
  const imageConversion = pageScript.indexOf("turn every png");
  const secretGeneration = pageScript.indexOf("generate a");
  const processSorting = pageScript.indexOf("show the five");
  const folderSizing = pageScript.indexOf("show the ten");
  const rustSearch = pageScript.indexOf("find all rust files");
  const largestFiles = pageScript.indexOf("show the 10");

  assert.ok(imageConversion < secretGeneration);
  assert.ok(secretGeneration < processSorting);
  assert.ok(processSorting < folderSizing);
  assert.ok(folderSizing < rustSearch);
  assert.ok(rustSearch < largestFiles);
  assert.doesNotMatch(pageScript, /expenses\.csv|mentioning hosting/);
});

test("labels the rotating hero as examples until the live sandbox opens", () => {
  assert.match(pageMarkup, />jst \/ examples</);
  assert.match(pageMarkup, />plain english → shell</);
  assert.doesNotMatch(pageMarkup, />jst \/ live demo</);
  assert.match(pageScript, /terminalTitle\.textContent = "jst \/ live demo"/);
  assert.match(pageScript, /terminalRuntime\.textContent = "wasm · linux"/);
});
