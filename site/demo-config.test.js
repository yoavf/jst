import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import viteConfig from "../vite.config.js";
import { statsTotalSizeStep } from "../docs/stats-display.js";

const [pageScript, pageStyles, runtimeBundle, sandboxBundle, sandboxMarkup] =
  await Promise.all([
  readFile(new URL("../docs/script.js", import.meta.url), "utf8"),
  readFile(new URL("../docs/styles.css", import.meta.url), "utf8"),
  readFile(new URL("../docs/assets/demo-runtime.js", import.meta.url), "utf8"),
  readFile(new URL("../docs/assets/demo-sandbox.js", import.meta.url), "utf8"),
  readFile(new URL("../docs/demo-sandbox.html", import.meta.url), "utf8"),
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

test("loads one versioned browser toolbox bundle in both page and worker", () => {
  const pageImport = pageScript.match(/assets\/(demo-command-v\d+\.js)/);

  assert.ok(pageImport, "page should import a versioned toolbox filename");
  assert.match(
    sandboxBundle,
    new RegExp(`from "\\./${pageImport[1]}"`),
    "worker should import the same versioned toolbox filename",
  );
});

test("cache-busts every layer of the sandbox runtime", () => {
  assert.match(pageMarkup, /script\.js\?v=44/);
  assert.match(pageScript, /demo-runtime\.js\?v=4/);
  assert.match(runtimeBundle, /demo-sandbox\.html\?v=15/);
  assert.match(sandboxMarkup, /demo-sandbox\.js\?v=15/);
  assert.match(sandboxBundle, /demo-sandbox\.js\?v=15/);
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

test("ships cross-platform examples returned by the JST CLI", () => {
  assert.match(pageScript, /ps aux/);
  assert.match(pageScript, /sort -nrk 4/);
  assert.match(pageScript, /du -sk/);
  assert.match(pageScript, /for file in /);
  assert.match(pageScript, /screenshots\/\*\.png/);
  assert.match(pageScript, /openssl rand/);
  assert.match(pageScript, /cut -c1-32/);
  assert.match(pageScript, /openssl dgst -sha256/);
  assert.doesNotMatch(pageScript, /--sort=-%mem|--max-depth|sha256sum/);
});

test("leads with the most interesting examples and omits the expenses search", () => {
  const imageConversion = pageScript.indexOf("turn every png");
  const secretGeneration = pageScript.indexOf("generate a");
  const processSorting = pageScript.indexOf("show the five");
  const folderSizing = pageScript.indexOf("show the ten");
  const rustSearch = pageScript.indexOf("find all rust files");
  const largestFiles = pageScript.indexOf("show the 10");

  assert.ok(secretGeneration < imageConversion);
  assert.ok(imageConversion < processSorting);
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

test("starts by explaining the already-rendered first example", () => {
  assert.match(
    pageScript,
    /window\.requestAnimationFrame\(\(\) => \{\s*exampleIndex = 0;\s*explainRenderedExample\(exampleIndex\);/,
  );
  assert.match(pageScript, /animateExplanation\(example, run, false\)/);
});
