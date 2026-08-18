import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import viteConfig from "../vite.config.js";
import { publicStatsDays, statsTotalSizeStep } from "../docs/stats-display.js";

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
  assert.match(pageMarkup, /styles\.css\?v=30/);
  assert.match(pageMarkup, /script\.js\?v=47/);
  assert.match(pageScript, /demo-runtime\.js\?v=4/);
  assert.match(runtimeBundle, /demo-sandbox\.html\?v=15/);
  assert.match(sandboxMarkup, /demo-sandbox\.js\?v=15/);
  assert.match(sandboxBundle, /demo-sandbox\.js\?v=15/);
});

test("offers mobile visitors a save-for-later install flow", () => {
  assert.match(pageMarkup, /Save <code>jst<\/code> for later/);
  assert.match(pageMarkup, /Email myself the link/);
  assert.match(pageMarkup, /mailto:\?subject=Install%20jst/);
  assert.match(pageScript, /navigator\.share\(REMINDER_SHARE_DATA\)/);
  assert.match(pageScript, /navigator\.clipboard\.writeText\(REMINDER_URL\)/);
  assert.match(pageScript, /window\.clearTimeout\(reminderCopyResetTimer\)/);
  assert.match(pageScript, /window\.visualViewport\.width/);
  assert.match(pageStyles, /--reminder-viewport-width/);
  assert.match(pageStyles, /@media \(max-width: 640px\)[\s\S]*\.mobile-reminder\s*{[\s\S]*display: block;/);
});

test("links macOS visitors to the on-device Apple Intelligence instructions", () => {
  assert.match(pageMarkup, /class="usage-footnote"/);
  assert.match(pageMarkup, /On macOS 27 beta, Apple Intelligence runs locally—private, but experimental/);
  assert.match(pageMarkup, /href="https:\/\/github\.com\/yoavf\/jst#apple-intelligence-macos-27-beta"/);
  assert.match(pageStyles, /\.usage-footnote\s*{[\s\S]*grid-column: 2 \/ -1;[\s\S]*font-size: 0\.78rem;/);
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

test("public stats chart begins at its published start date", () => {
  assert.deepEqual(
    publicStatsDays(
      [
        { date: "2026-08-15", count: 3 },
        { date: "2026-08-16", count: 5 },
        { date: "2026-08-17", count: 8 },
      ],
      "2026-08-16",
      [
        { date: "2026-08-10", count: 2 },
        { date: "2026-08-15", count: 3 },
      ],
    ),
    [
      { date: "2026-08-10", count: 2 },
      { date: "2026-08-15", count: 3 },
      { date: "2026-08-16", count: 5 },
      { date: "2026-08-17", count: 8 },
    ],
  );
});

test("pre-launch stats preserve every historical query", () => {
  const prelaunch = pageScript.matchAll(
    /date: "2026-08-1[0-5]", count: (\d+)/g,
  );
  assert.equal(
    [...prelaunch].reduce((total, match) => total + Number(match[1]), 0),
    343,
  );
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
