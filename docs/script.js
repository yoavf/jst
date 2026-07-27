import { JST_HELP, JST_VERSION, parseJstInvocation } from "./assets/demo-cli.js";
import { isAllowedDemoCommand } from "./assets/demo-command.js?v=3";

const examples = [
  {
    requestParts: [
      { text: "jst " },
      { text: "find all rust files", map: "files" },
      { text: " in projects", map: "place" },
    ],
    resultParts: [
      { text: "find", map: "files" },
      { text: " projects", map: "place" },
      { text: " -type f", map: "files" },
      { text: " -name '*.rs'", map: "files" },
    ],
    mapOrder: ["files", "place"],
  },
  {
    requestParts: [
      { text: "jst " },
      { text: "show lines mentioning hosting", map: "search" },
      { text: " in downloads/expenses.csv", map: "file" },
    ],
    resultParts: [
      { text: "grep hosting", map: "search" },
      { text: " downloads/expenses.csv", map: "file" },
    ],
    mapOrder: ["search", "file"],
  },
  {
    requestParts: [
      { text: "jst " },
      { text: "show the 10", map: "limit" },
      { text: " largest", map: "sort" },
      { text: " files in this folder", map: "measure" },
    ],
    resultParts: [
      { text: "ls -lh", map: "measure" },
      { text: "S", map: "sort" },
      { text: " | head -n 10", map: "limit" },
    ],
    mapOrder: ["limit", "sort", "measure"],
  },
  {
    requestParts: [
      { text: "jst " },
      { text: "checksum", map: "action" },
      { text: " the README", map: "file" },
    ],
    resultParts: [
      { text: "sha256sum", map: "action" },
      { text: " README.md", map: "file" },
    ],
    mapOrder: ["action", "file"],
  },
];

const requestElement = document.querySelector("#request");
const resultElement = document.querySelector("#result");
const translationElement = document.querySelector(".translation");
const resultLine = document.querySelector(".terminal-line--result");
const anotherButton = document.querySelector(".another-example");
const copyButton = document.querySelector(".install-command");
const copyState = document.querySelector(".copy-state");
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
const cursorElement = document.querySelector(".terminal-cursor");
const spinnerElement = document.querySelector(".terminal-spinner");
const mapElement = document.querySelector(".translation-map");
const privacyDialog = document.querySelector("#privacy-dialog");
const privacyOpenButtons = document.querySelectorAll("[data-privacy-open]");
const privacyCloseButton = document.querySelector("[data-privacy-close]");
const tryDemoButton = document.querySelector(".try-demo");
const tryDemoLabel = document.querySelector(".try-demo-label");
const demoForm = document.querySelector("#demo-form");
const demoInput = document.querySelector("#demo-input");
const demoSubmit = document.querySelector(".demo-submit");
const demoStatus = document.querySelector("#demo-status");
const demoSession = document.querySelector("#demo-session");
const demoScrollback = document.querySelector("#demo-scrollback");
const demoClearButton = document.querySelector("#demo-clear");
const demoResetButton = document.querySelector("#demo-reset");
const terminalTitle = document.querySelector("#terminal-title");
const terminalRuntime = document.querySelector("#terminal-runtime");
const terminalSessionActions = document.querySelector("#terminal-session-actions");
const exampleRequestLine = document.querySelector(".demo-example-request");
const demoDialog = document.querySelector("#demo-dialog");
const demoDialogMount = document.querySelector("#demo-dialog-mount");
const demoDialogClose = document.querySelector("[data-demo-close]");
const translationAnchor = document.createComment("JST terminal home");
translationElement.before(translationAnchor);

let exampleIndex = 0;
let animationRun = 0;
let autoRotateTimer = null;
let spinnerTimer = null;
let demoRuntime = null;
let browserIdCache = null;
let demoHistory = [];
let demoHistoryIndex = 0;
let demoDraft = "";
let demoActivationRun = 0;
let demoRequestController = null;
let demoReviewCancel = null;
const AUTO_ROTATE_DELAY = 5000;
const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const DEFAULT_DEMO_REQUEST = "list files in this folder";
const IS_LOCAL_DEMO = ["127.0.0.1", "localhost"].includes(location.hostname);
const DEMO_URL = IS_LOCAL_DEMO
  ? "/api/jst-demo"
  : "https://jst-server.fly.dev/demo";
const SERVER_STATUS_URL = IS_LOCAL_DEMO
  ? "/api/jst-status"
  : "https://jst-server.fly.dev/status";
const BROWSER_ID_KEY = "jst-demo-browser-id";
const BROWSER_ID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const SANDBOX_PROGRESS = {
  runtime: "starting the WASI runtime…",
  package: "loading 50+ Rust Linux tools…",
  shell: "mounting the disposable workspace…",
  ready: "sandbox ready",
};
const RICKROLL_MARKER = "JST_RICKROLL_V1";
const QUEST_COMPLETE_MARKER = "JST_QUEST_COMPLETE_V1";
const RICKROLL_FRAMES = [
  String.raw`╭──────── RICKROLL.EXE ────────╮
│              ♪               │
│          .-""""-.             │
│         /  •  •  \      🎤    │
│         |   ▿    |      /     │
│          \  ─   /   \o/       │
│           /|  |\     |        │
│          / |  | \   / \       │
╰───────────────────────────────╯`,
  String.raw`╭──────── RICKROLL.EXE ────────╮
│          ♪                   │
│          .-""""-.             │
│         /  •  •  \   🎤       │
│         |   ▿    |  /         │
│          \  ─   /  o/         │
│           /|  |\  /|          │
│          / |  | \ / \         │
╰───────────────────────────────╯`,
  String.raw`╭──────── RICKROLL.EXE ────────╮
│                    ♪         │
│          .-""""-.             │
│         /  •  •  \       🎤   │
│         |   ▿    |       \    │
│          \  ─   /       \o    │
│           /|  |\         |\   │
│          / |  | \       / \   │
╰───────────────────────────────╯`,
  String.raw`╭──────── RICKROLL.EXE ────────╮
│              ♫               │
│          .-""""-.             │
│         /  •  •  \    🎤      │
│         |   ▿    |    /       │
│          \  ─   /   _o_       │
│           /|  |\     |        │
│          / |  | \   / \       │
╰───────────────────────────────╯`,
];

function wait(milliseconds) {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}

async function typeText(element, value, delay, run) {
  element.textContent = "";

  for (const character of value) {
    if (run !== animationRun) return false;
    element.textContent += character;
    await wait(delay);
  }

  return true;
}

function exampleText(parts) {
  return parts.map(({ text }) => text).join("");
}

function renderParts(element, parts, className) {
  element.replaceChildren(
    ...parts.map(({ text, map }) => {
      const part = document.createElement("span");
      part.textContent = text;
      part.className = className;
      if (map) part.dataset.map = map;
      return part;
    }),
  );
}

function clearMap() {
  mapElement.replaceChildren();
  mapElement.classList.remove("is-drawing");
  translationElement.querySelectorAll(".is-mapping").forEach((part) => {
    part.classList.remove("is-mapping");
  });
}

function drawMap(map) {
  clearMap();

  const source = requestElement.querySelector(`[data-map="${map}"]`);
  const targets = resultElement.querySelectorAll(`[data-map="${map}"]`);
  if (!source || targets.length === 0) return;

  const containerRect = translationElement.getBoundingClientRect();
  const sourceRect = source.getBoundingClientRect();
  source.classList.add("is-mapping");

  mapElement.setAttribute("viewBox", `0 0 ${containerRect.width} ${containerRect.height}`);

  targets.forEach((target) => {
    const targetRect = target.getBoundingClientRect();
    const startX = sourceRect.left + sourceRect.width / 2 - containerRect.left;
    const startY = sourceRect.bottom - containerRect.top + 4;
    const endX = targetRect.left + targetRect.width / 2 - containerRect.left;
    const endY = targetRect.top - containerRect.top - 5;
    const bend = Math.max(16, Math.abs(endY - startY) * 0.48);
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");

    path.setAttribute(
      "d",
      `M ${startX} ${startY} C ${startX} ${startY + bend}, ${endX} ${endY - bend}, ${endX} ${endY}`,
    );
    mapElement.append(path);
    path.style.setProperty("--path-length", path.getTotalLength());
    target.classList.add("is-mapping");

    const dot = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    dot.setAttribute("cx", endX);
    dot.setAttribute("cy", endY);
    dot.setAttribute("r", "2.5");
    mapElement.append(dot);
  });

  window.requestAnimationFrame(() => mapElement.classList.add("is-drawing"));
}

async function assembleResult(example, run) {
  renderParts(resultElement, example.resultParts, "result-part");
  resultLine.classList.add("is-visible");

  for (const map of example.mapOrder) {
    if (run !== animationRun) return false;
    drawMap(map);
    await wait(80);
    resultElement.querySelectorAll(`[data-map="${map}"]`).forEach((part) => {
      part.classList.add("is-visible");
    });
    await wait(520);
  }

  clearMap();
  return true;
}

function resetAutoRotate() {
  if (autoRotateTimer) clearTimeout(autoRotateTimer);
  autoRotateTimer = setTimeout(() => {
    exampleIndex = (exampleIndex + 1) % examples.length;
    showExample(exampleIndex);
  }, AUTO_ROTATE_DELAY);
}

function startSpinner() {
  let frameIndex = 0;
  spinnerElement.textContent = SPINNER_FRAMES[0];
  spinnerTimer = setInterval(() => {
    frameIndex = (frameIndex + 1) % SPINNER_FRAMES.length;
    spinnerElement.textContent = SPINNER_FRAMES[frameIndex];
  }, 80);
}

function stopSpinner() {
  if (spinnerTimer) {
    clearInterval(spinnerTimer);
    spinnerTimer = null;
  }
  spinnerElement.textContent = "";
}

function createBrowserId() {
  if (typeof crypto.randomUUID === "function") return crypto.randomUUID();

  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

function browserId() {
  if (browserIdCache) return browserIdCache;

  try {
    const saved = localStorage.getItem(BROWSER_ID_KEY);
    if (saved && BROWSER_ID_PATTERN.test(saved)) {
      browserIdCache = saved;
      return saved;
    }
    browserIdCache = createBrowserId();
    localStorage.setItem(BROWSER_ID_KEY, browserIdCache);
    return browserIdCache;
  } catch {
    browserIdCache = createBrowserId();
    return browserIdCache;
  }
}

function safeSandboxText(value) {
  let safe = "";
  for (const character of value) {
    const code = character.codePointAt(0);
    const unsafe =
      (code < 32 && character !== "\n" && character !== "\t") ||
      code === 127 ||
      code === 0x061c ||
      code === 0x200e ||
      code === 0x200f ||
      (code >= 0x202a && code <= 0x202e) ||
      (code >= 0x2066 && code <= 0x2069);
    safe += unsafe ? `\\u{${code.toString(16)}}` : character;
  }
  return safe;
}

function renderTerminalRickroll(output) {
  output.classList.add("demo-rickroll");
  const animation = document.createElement("span");
  animation.className = "demo-rickroll-frame";
  animation.setAttribute("aria-hidden", "true");
  const accessible = document.createElement("span");
  accessible.className = "visually-hidden";
  accessible.textContent = "ASCII rickroll animation.";
  output.replaceChildren(animation, accessible);

  let frame = 0;
  let ticks = 0;
  const draw = () => {
    animation.textContent = RICKROLL_FRAMES[frame];
    frame = (frame + 1) % RICKROLL_FRAMES.length;
  };
  draw();
  if (reduceMotion.matches) return;

  const timer = window.setInterval(() => {
    if (!output.isConnected || ticks >= 31) {
      window.clearInterval(timer);
      return;
    }
    ticks += 1;
    draw();
  }, 210);
}

function renderQuestComplete(output) {
  output.classList.add("demo-quest-complete");

  const boot = document.createElement("span");
  boot.className = "demo-quest-boot";
  boot.textContent = [
    "[  OK  ] recovered pid 31337",
    "[  OK  ] exit status 0",
  ].join("\n");

  const title = document.createElement("strong");
  title.className = "demo-quest-title";
  title.textContent = "You've mastered jst.";

  const prompt = document.createElement("span");
  prompt.className = "demo-quest-prompt";
  prompt.textContent = "Now take it with you.";

  const install = document.createElement("span");
  install.className = "demo-quest-install";
  const releases = document.createElement("a");
  releases.href = "https://github.com/yoavf/jst/releases/latest";
  releases.target = "_blank";
  releases.rel = "noopener noreferrer";

  const platform =
    navigator.userAgentData?.platform ||
    navigator.platform ||
    navigator.userAgent;
  const isMac = /mac/i.test(platform);
  if (isMac) {
    const shellPrompt = document.createElement("span");
    shellPrompt.setAttribute("aria-hidden", "true");
    shellPrompt.textContent = "$ ";
    const command = document.createElement("code");
    command.textContent = "brew install yoavf/tap/jst";
    install.append(shellPrompt, command);
    releases.textContent = "Other platforms: GitHub Releases →";
  } else {
    releases.className = "demo-quest-primary-link";
    releases.textContent = "Download jst from GitHub Releases →";
    const macHint = document.createElement("span");
    macHint.className = "demo-quest-mac-hint";
    macHint.textContent = "macOS: ";
    const command = document.createElement("code");
    command.textContent = "brew install yoavf/tap/jst";
    macHint.append(command);
    install.append(releases, macHint);
  }

  output.replaceChildren(boot, title, prompt, install);
  if (isMac) output.append(releases);
}

function appendDemoLocalOutput(input, output, isError = false) {
  const entry = appendDemoEntry(input);
  entry.stopSpinner();
  entry.entry.querySelector(".demo-entry-command")?.remove();
  entry.output.textContent = output;
  if (isError) entry.entry.classList.add("demo-entry-error");
  return entry;
}

function invocationError(message) {
  return `error: ${message}\n\nFor more information, try '--help'.`;
}

function formatDemoServerStatus(status) {
  const lines = [
    `Server: ${status.status || "unknown"}`,
    `Primary model: ${status.model || "unknown"}`,
    `Fallback model: ${status.fallback_model || "none"}`,
  ];
  if (status.usage) {
    lines.push(`Calls today: ${status.usage.calls_today}`);
    lines.push(`Calls all time: ${status.usage.calls_total}`);
  } else {
    lines.push("Usage stats: unavailable");
  }
  return lines.join("\n");
}

function requestDemoApproval(entry, explanation) {
  return new Promise((resolve) => {
    const review = document.createElement("div");
    review.className = "demo-review";
    review.tabIndex = -1;

    const question = document.createElement("p");
    question.className = "demo-review-question";
    question.textContent = "Run this command?";

    const actions = document.createElement("div");
    actions.className = "demo-review-actions";
    const yes = document.createElement("button");
    yes.type = "button";
    yes.textContent = "[y] yes";
    const no = document.createElement("button");
    no.type = "button";
    no.textContent = "[n] no";
    const why = document.createElement("button");
    why.type = "button";
    why.textContent = "[w] why";
    actions.append(yes, no, why);

    const details = document.createElement("p");
    details.className = "demo-review-explanation";
    details.hidden = true;
    details.textContent = safeSandboxText(
      explanation?.trim() || "No additional explanation was returned.",
    );
    review.append(question, actions, details);
    entry.entry.append(review);
    scrollDemoToBottom();

    let settled = false;
    const finish = (approved) => {
      if (settled) return;
      settled = true;
      if (demoReviewCancel === cancel) demoReviewCancel = null;
      review.remove();
      resolve(approved);
    };
    const cancel = () => finish(false);
    const showWhy = () => {
      details.hidden = false;
      why.disabled = true;
      scrollDemoToBottom();
    };
    demoReviewCancel = cancel;
    yes.addEventListener("click", () => finish(true));
    no.addEventListener("click", () => finish(false));
    why.addEventListener("click", showWhy);
    review.addEventListener("keydown", (event) => {
      if (event.key.toLowerCase() === "y") finish(true);
      else if (event.key.toLowerCase() === "n" || event.key === "Escape") finish(false);
      else if (event.key.toLowerCase() === "w") showWhy();
    });
    yes.focus();
  });
}

function setDemoBusy(busy) {
  demoInput.disabled = busy;
  demoSubmit.disabled = busy;
  demoClearButton.disabled = busy;
  demoResetButton.disabled = busy;
  demoForm.hidden = busy;
  translationElement.classList.toggle("is-translating", busy);
}

function scrollDemoToBottom() {
  window.requestAnimationFrame(() => {
    demoScrollback.scrollTop = demoScrollback.scrollHeight;
  });
}

function appendDemoMotd() {
  const motd = document.createElement("div");
  motd.className = "demo-motd";
  motd.textContent = [
    "JST sandbox 0.7",
    "Try jst --help for options.",
  ].join("\n");
  demoScrollback.append(motd);
}

function clearDemoScrollback() {
  demoScrollback.replaceChildren();
  appendDemoMotd();
  scrollDemoToBottom();
}

function appendDemoEntry(input) {
  const entry = document.createElement("article");
  entry.className = "demo-entry is-pending";

  const request = document.createElement("div");
  request.className = "demo-entry-request";
  const prompt = demoForm.querySelector(".terminal-shell-prompt").cloneNode(true);
  const requestCode = document.createElement("code");
  requestCode.textContent = input ? `jst ${input}` : "jst";
  request.append(prompt, requestCode);

  const command = document.createElement("div");
  command.className = "demo-entry-command";
  const arrow = document.createElement("span");
  arrow.className = "demo-entry-arrow";
  arrow.textContent = "→";
  arrow.setAttribute("aria-hidden", "true");
  const spinner = document.createElement("span");
  spinner.className = "demo-entry-spinner";
  spinner.setAttribute("aria-hidden", "true");
  const commandCode = document.createElement("code");
  command.append(arrow, spinner, commandCode);

  const output = document.createElement("pre");
  output.className = "demo-entry-output";
  entry.append(request, command, output);
  demoScrollback.append(entry);
  scrollDemoToBottom();

  let frameIndex = 0;
  spinner.textContent = SPINNER_FRAMES[frameIndex];
  const entrySpinner = window.setInterval(() => {
    frameIndex = (frameIndex + 1) % SPINNER_FRAMES.length;
    spinner.textContent = SPINNER_FRAMES[frameIndex];
  }, 80);

  return {
    commandCode,
    entry,
    output,
    stopSpinner() {
      window.clearInterval(entrySpinner);
      spinner.remove();
      entry.classList.remove("is-pending");
    },
  };
}

function reportSandboxProgress(stage) {
  if (!translationElement.classList.contains("is-interactive")) return;
  const message = stage === "ready" ? "session ready" : SANDBOX_PROGRESS[stage];
  if (!message) return;
  demoStatus.textContent = message;
}

function openDemoDialog() {
  if (!demoDialog || !demoDialogMount || demoDialog.open) return;
  demoDialogMount.append(translationElement);
  demoDialog.showModal();
  document.documentElement.classList.add("demo-dialog-open");
}

function restoreDemoPreview() {
  demoActivationRun += 1;
  demoReviewCancel?.();
  demoReviewCancel = null;
  demoRequestController?.abort();
  demoRequestController = null;
  demoRuntime?.destroy();
  demoRuntime = null;
  setDemoBusy(false);
  demoHistory = [];
  demoHistoryIndex = 0;
  demoDraft = "";
  demoInput.value = "";
  demoSession.hidden = true;
  demoScrollback.replaceChildren();
  exampleRequestLine.hidden = false;
  terminalTitle.textContent = "jst / live demo";
  terminalRuntime.hidden = false;
  terminalSessionActions.hidden = true;
  tryDemoButton.disabled = false;
  tryDemoLabel.textContent = "try it now";
  translationElement.classList.remove(
    "is-activating",
    "is-interactive",
    "is-loading",
    "is-translating",
  );
  translationAnchor.after(translationElement);
  document.documentElement.classList.remove("demo-dialog-open");
  showExample(exampleIndex, false);
}

function closeDemoDialog() {
  if (demoDialog?.open) demoDialog.close();
}

function prefillDemoInput() {
  demoInput.value = DEFAULT_DEMO_REQUEST;
  demoInput.setSelectionRange(demoInput.value.length, demoInput.value.length);
}

async function activateDemo() {
  if (!tryDemoButton || !demoForm) return;

  openDemoDialog();
  const activationRun = ++demoActivationRun;
  if (autoRotateTimer) {
    clearTimeout(autoRotateTimer);
    autoRotateTimer = null;
  }
  animationRun += 1;
  clearMap();
  stopSpinner();
  translationElement.classList.remove("is-loading");
  translationElement.classList.add("is-activating");
  translationElement.setAttribute("aria-label", "Starting the JST browser sandbox");
  requestElement.textContent = "starting sandbox…";
  cursorElement.classList.remove("is-hidden");
  resultLine.classList.remove("is-visible");
  resultElement.textContent = "";
  tryDemoButton.disabled = true;
  demoStatus.textContent = "";

  try {
    const { DemoRuntime } = await import("./assets/demo-runtime.js?v=2");
    demoRuntime?.destroy();
    demoRuntime = new DemoRuntime({ onProgress: reportSandboxProgress });
    await demoRuntime.boot();
    if (activationRun !== demoActivationRun || !demoDialog?.open) {
      demoRuntime?.destroy();
      demoRuntime = null;
      return;
    }

    exampleRequestLine.hidden = true;
    resultLine.classList.remove("is-visible");
    resultLine.removeAttribute("aria-hidden");
    resultElement.textContent = "";
    demoSession.hidden = false;
    translationElement.setAttribute("aria-label", "Interactive JST browser demo");
    terminalTitle.textContent = "guest@jst: ~/playground";
    terminalRuntime.hidden = true;
    terminalSessionActions.hidden = false;
    clearDemoScrollback();
    translationElement.classList.add("is-interactive");
    demoStatus.textContent = "";
    prefillDemoInput();
    demoInput.focus();
  } catch (error) {
    if (activationRun !== demoActivationRun) return;
    demoRuntime?.destroy();
    demoRuntime = null;
    const message =
      error instanceof Error ? error.message : "The browser sandbox could not start.";
    requestElement.textContent = message;
    cursorElement.classList.add("is-hidden");
    translationElement.setAttribute("aria-label", message);
    tryDemoButton.disabled = false;
    tryDemoLabel.textContent = "try again";
  } finally {
    translationElement.classList.remove("is-activating");
  }
}

async function submitDemo(event) {
  event.preventDefault();
  const invocationText = demoInput.value.trim();
  if (!demoRuntime) return;
  const invocation = parseJstInvocation(invocationText);
  demoInput.value = "";

  if (invocation.error) {
    appendDemoLocalOutput(invocationText, invocationError(invocation.error), true);
    scrollDemoToBottom();
    return;
  }
  if (invocation.action === "help") {
    appendDemoLocalOutput(invocationText, JST_HELP);
    scrollDemoToBottom();
    return;
  }
  if (invocation.action === "version") {
    appendDemoLocalOutput(invocationText, `jst ${JST_VERSION} (browser playground)`);
    scrollDemoToBottom();
    return;
  }
  if (invocation.action === "status") {
    demoHistory.push(invocationText);
    demoHistoryIndex = demoHistory.length;
    const entry = appendDemoLocalOutput(invocationText, "Checking server…");
    setDemoBusy(true);
    try {
      const response = await fetch(SERVER_STATUS_URL);
      const status = await response.json().catch(() => ({}));
      if (!response.ok) throw new Error("the jst server is having trouble");
      entry.output.textContent = formatDemoServerStatus(status);
    } catch (error) {
      entry.entry.classList.add("demo-entry-error");
      entry.output.textContent = `jst: ${
        error instanceof Error ? error.message : "could not reach the jst server"
      }`;
    } finally {
      setDemoBusy(false);
      demoInput.focus();
      scrollDemoToBottom();
    }
    return;
  }

  const input = invocation.input;
  if (new TextEncoder().encode(input).length > 280) {
    demoStatus.textContent = "keep the request under 280 bytes";
    return;
  }

  demoHistory.push(invocationText);
  demoHistoryIndex = demoHistory.length;
  demoDraft = "";
  const entry = appendDemoEntry(invocationText);
  setDemoBusy(true);
  demoStatus.textContent = "translating with real jst…";
  const requestController = new AbortController();
  demoRequestController?.abort();
  demoRequestController = requestController;

  try {
    const response = await fetch(DEMO_URL, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-jst-browser-id": browserId(),
      },
      body: JSON.stringify({
        input,
        os: "linux",
        interactive: invocation.interactive,
      }),
      signal: requestController.signal,
    });
    const body = await response.json().catch(() => ({}));
    if (typeof body.command === "string" && body.command.trim()) {
      entry.stopSpinner();
      entry.commandCode.textContent = safeSandboxText(body.command);
    }
    if (!response.ok) {
      throw new Error(
        body.error ||
          (response.status === 429
            ? "The demo limit was reached. Try again later."
            : "JST could not translate that right now."),
      );
    }

    if (!isAllowedDemoCommand(body.command)) {
      throw new Error(
        "Not in the browser toolbox yet — but now we know what to add.",
      );
    }
    entry.stopSpinner();
    entry.commandCode.textContent = body.command;
    translationElement.setAttribute(
      "aria-label",
      `JST translated ${input} to ${body.command} and ran it in the browser sandbox`,
    );

    if (!body.matches_request || body.command.trim().startsWith("#")) {
      entry.entry.classList.add("demo-entry-error");
      entry.output.textContent = "jst: I could not confidently match that request. Try rephrasing it.";
      demoStatus.textContent = "";
      return;
    }

    if (invocation.dry) {
      demoStatus.textContent = "";
      return;
    }

    if (invocation.interactive) {
      const approved = await requestDemoApproval(entry, body.explanation);
      if (!approved) {
        entry.output.textContent = "Aborted.";
        demoStatus.textContent = "";
        return;
      }
    }

    demoStatus.textContent = `running ${body.command.split(/\s/, 1)[0]} in the sandbox…`;
    const output = await demoRuntime.run(body.command);
    const stdout = safeSandboxText(output.stdout || "");
    const stderr = safeSandboxText(output.stderr || "");
    const redirectedStdout = safeSandboxText(output.redirectedStdout || "");
    const commandOutput = `${stdout}${stderr}`;
    if (
      commandOutput.trim() === RICKROLL_MARKER ||
      redirectedStdout.trim() === RICKROLL_MARKER
    ) {
      renderTerminalRickroll(entry.output);
    } else if (
      commandOutput.trim() === QUEST_COMPLETE_MARKER ||
      redirectedStdout.trim() === QUEST_COMPLETE_MARKER
    ) {
      renderQuestComplete(entry.output);
    } else {
      entry.output.textContent = commandOutput.trim()
        ? commandOutput.trimEnd()
        : `(no output; exited ${output.code})`;
    }
    if (output.code !== 0) entry.entry.classList.add("demo-entry-error");

    demoStatus.textContent = "";
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return;
    entry.stopSpinner();
    entry.entry.classList.add("demo-entry-error");
    const message =
      error instanceof Error ? error.message : "The demo hit a temporary problem.";
    entry.output.textContent = `jst: ${message}`;
    demoStatus.textContent = demoRuntime?.isActive()
      ? ""
      : "sandbox reset · use reset to start again";
  } finally {
    if (demoRequestController === requestController) {
      demoRequestController = null;
    }
    entry.stopSpinner();
    setDemoBusy(false);
    if (demoDialog?.open) demoInput.focus();
    scrollDemoToBottom();
  }
}

async function resetDemoSession() {
  if (!demoRuntime || demoInput.disabled) return;
  setDemoBusy(true);
  demoStatus.textContent = "resetting the disposable filesystem…";
  try {
    demoRuntime.destroy();
    await demoRuntime.boot();
    demoHistory = [];
    demoHistoryIndex = 0;
    demoDraft = "";
    prefillDemoInput();
    clearDemoScrollback();
    demoStatus.textContent = "";
  } catch (error) {
    demoStatus.textContent =
      error instanceof Error ? error.message : "The browser sandbox could not restart.";
  } finally {
    setDemoBusy(false);
    demoInput.focus();
  }
}

function navigateDemoHistory(direction) {
  if (demoHistory.length === 0) return;
  if (demoHistoryIndex === demoHistory.length) demoDraft = demoInput.value;
  demoHistoryIndex = Math.max(
    0,
    Math.min(demoHistory.length, demoHistoryIndex + direction),
  );
  demoInput.value =
    demoHistoryIndex === demoHistory.length
      ? demoDraft
      : demoHistory[demoHistoryIndex];
  demoInput.setSelectionRange(demoInput.value.length, demoInput.value.length);
}

async function showExample(index, animate = true) {
  const example = examples[index];
  const request = exampleText(example.requestParts);
  const result = exampleText(example.resultParts);
  const run = ++animationRun;
  translationElement.setAttribute(
    "aria-label",
    `JST turns ${request} into the shell command ${result}`,
  );

  cursorElement.classList.remove("is-hidden");
  stopSpinner();
  translationElement.classList.remove("is-loading");
  clearMap();

  if (!animate || reduceMotion.matches) {
    requestElement.textContent = request;
    resultElement.textContent = result;
    resultLine.classList.add("is-visible");
    cursorElement.classList.add("is-hidden");
    resetAutoRotate();
    return;
  }

  requestElement.textContent = "";
  resultElement.textContent = "";
  resultLine.classList.remove("is-visible");

  await wait(220);
  const requestFinished = await typeText(requestElement, request, 24, run);
  if (!requestFinished || run !== animationRun) return;

  renderParts(requestElement, example.requestParts, "request-part");
  cursorElement.classList.add("is-hidden");
  translationElement.classList.add("is-loading");
  startSpinner();

  await wait(720);
  if (run !== animationRun) return;
  stopSpinner();
  translationElement.classList.remove("is-loading");
  const resultFinished = await assembleResult(example, run);
  if (resultFinished) resetAutoRotate();
}

anotherButton?.addEventListener("click", () => {
  if (autoRotateTimer) {
    clearTimeout(autoRotateTimer);
    autoRotateTimer = null;
  }
  exampleIndex = (exampleIndex + 1) % examples.length;
  showExample(exampleIndex);
});

tryDemoButton?.addEventListener("click", activateDemo);
demoDialogClose?.addEventListener("click", closeDemoDialog);
demoDialog?.addEventListener("cancel", (event) => {
  event.preventDefault();
  closeDemoDialog();
});
demoDialog?.addEventListener("click", (event) => {
  if (event.target === demoDialog) closeDemoDialog();
});
demoDialog?.addEventListener("close", restoreDemoPreview);
demoForm?.addEventListener("submit", submitDemo);
demoClearButton?.addEventListener("click", () => {
  clearDemoScrollback();
  demoStatus.textContent = "";
  demoInput.focus();
});
demoResetButton?.addEventListener("click", resetDemoSession);
demoInput?.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && !event.isComposing) {
    event.preventDefault();
    demoForm.requestSubmit();
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    navigateDemoHistory(-1);
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    navigateDemoHistory(1);
  } else if (event.ctrlKey && event.key.toLowerCase() === "c") {
    event.preventDefault();
    demoInput.value = "";
    demoHistoryIndex = demoHistory.length;
    demoDraft = "";
    demoStatus.textContent = "";
  } else if (event.ctrlKey && event.key.toLowerCase() === "l") {
    event.preventDefault();
    clearDemoScrollback();
    demoStatus.textContent = "";
  }
});
translationElement?.addEventListener("click", (event) => {
  const target = event.target instanceof Element ? event.target : null;
  if (
    translationElement.classList.contains("is-interactive") &&
    !target?.closest("button, #demo-scrollback")
  ) {
    demoInput.focus();
  }
});

copyButton?.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(copyButton.dataset.copy);
    copyState.textContent = "Copied";
    window.setTimeout(() => {
      copyState.textContent = "copy";
    }, 1800);
  } catch {
    copyState.textContent = "select + copy";
  }
});

privacyOpenButtons.forEach((button) => {
  button.addEventListener("click", () => {
    privacyDialog?.showModal();
  });
});

privacyCloseButton?.addEventListener("click", () => {
  privacyDialog?.close();
});

privacyDialog?.addEventListener("click", (event) => {
  if (event.target === privacyDialog) privacyDialog.close();
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && privacyDialog?.open) {
    event.preventDefault();
    privacyDialog.close();
  }
});

window.requestAnimationFrame(() => {
  exampleIndex = 0;
  showExample(exampleIndex, false);
});

window.addEventListener("pagehide", () => {
  demoRuntime?.destroy();
});

// --- Live usage stats -------------------------------------------------

const STATS_URL = "https://jst-server.fly.dev/stats";
const STATS_REFRESH_INTERVAL_MS = 60_000;
const MAX_COMMAND_BARS = 10;
const MONTH_NAMES = [
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const statsSection = document.querySelector(".stats");
const statsTotalElement = document.querySelector("#stats-total");
const commandBarsElement = document.querySelector("#command-bars");
const dayBarsElement = document.querySelector("#day-bars");

function formatNumber(value) {
  return new Intl.NumberFormat("en-US").format(value);
}

function formatRangeDay(date) {
  const [year, month, day] = date.split("-").map(Number);
  return `${MONTH_NAMES[month - 1]} ${day}, ’${String(year).slice(-2)}`;
}

function renderCommandBar({ command, count }, index, max) {
  const item = document.createElement("li");
  item.className = "command-bar";

  const label = document.createElement("span");
  label.className = "command-bar-label";
  label.textContent = command;

  const track = document.createElement("span");
  track.className = "command-bar-track";
  track.setAttribute("aria-hidden", "true");

  const fill = document.createElement("span");
  fill.className = "command-bar-fill";
  if (index === 0) fill.classList.add("command-bar-fill--top");
  fill.style.setProperty("--value", `${Math.max((count / max) * 100, 2)}%`);
  track.append(fill);

  const tally = document.createElement("span");
  tally.className = "command-bar-count";
  tally.textContent = formatNumber(count);

  const summary = document.createElement("span");
  summary.className = "visually-hidden";
  summary.textContent = `${command}: ${formatNumber(count)} runs`;

  item.append(label, track, tally, summary);
  return item;
}

function renderDayBar({ date, count }, index, days, max) {
  const bar = document.createElement("span");
  bar.className = "day-bar";
  if (index === days.length - 1) bar.classList.add("day-bar--today");
  bar.style.setProperty("--value", `${(count / max) * 100}%`);
  const tooltip = `${formatRangeDay(date)} · ${formatNumber(count)} ${count === 1 ? "query" : "queries"}`;
  bar.dataset.tooltip = tooltip;
  bar.setAttribute("role", "listitem");
  bar.setAttribute("aria-label", tooltip);
  bar.tabIndex = 0;
  return bar;
}

async function loadStats() {
  const response = await fetch(STATS_URL);
  if (!response.ok) throw new Error(`stats returned ${response.status}`);
  const stats = await response.json();

  statsTotalElement.textContent = formatNumber(stats.total);

  const top = (stats.top_commands || []).slice(0, MAX_COMMAND_BARS);
  if (top.length > 0) {
    commandBarsElement.replaceChildren(
      ...top.map((entry, index) => renderCommandBar(entry, index, top[0].count)),
    );
  }

  const allDays = stats.daily || [];
  const firstActiveDay = allDays.findIndex((day) => day.count > 0);
  const daysSinceLaunch = firstActiveDay === -1 ? allDays : allDays.slice(firstActiveDay);
  const days = daysSinceLaunch.slice(-30);
  if (days.length > 0) {
    const max = Math.max(...days.map((day) => day.count), 1);
    dayBarsElement.replaceChildren(
      ...days.map((day, index) => renderDayBar(day, index, days, max)),
    );
  }

  statsSection.hidden = false;
}

async function refreshStats() {
  try {
    await loadStats();
  } catch {
    // Stats are best-effort: keep the current view until the next refresh.
  } finally {
    window.setTimeout(refreshStats, STATS_REFRESH_INTERVAL_MS);
  }
}

if (statsSection) {
  refreshStats();
}
