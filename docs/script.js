const examples = [
  {
    requestParts: [
      { text: "jst " },
      { text: "find all files", map: "files" },
      { text: " bigger than 500 mb", map: "size" },
      { text: " in ~/downloads", map: "place" },
    ],
    resultParts: [
      { text: "find", map: "files" },
      { text: " ~/downloads", map: "place" },
      { text: " -type f", map: "files" },
      { text: " -size +500M", map: "size" },
    ],
    mapOrder: ["files", "size", "place"],
  },
  {
    requestParts: [
      { text: "jst " },
      { text: "show me what's using", map: "process" },
      { text: " port 3000", map: "port" },
    ],
    resultParts: [
      { text: "lsof", map: "process" },
      { text: " -i :3000", map: "port" },
    ],
    mapOrder: ["process", "port"],
  },
  {
    requestParts: [
      { text: "jst " },
      { text: "show the 10", map: "limit" },
      { text: " largest", map: "sort" },
      { text: " files in this folder", map: "measure" },
    ],
    resultParts: [
      { text: "du -ah .", map: "measure" },
      { text: " | sort -hr", map: "sort" },
      { text: " | head -n 10", map: "limit" },
    ],
    mapOrder: ["limit", "sort", "measure"],
  },
  {
    requestParts: [
      { text: "jst " },
      { text: "find every TODO", map: "search" },
      { text: " in rust files", map: "rust" },
    ],
    resultParts: [
      { text: "rg", map: "search" },
      { text: " --glob '*.rs'", map: "rust" },
      { text: " TODO", map: "search" },
    ],
    mapOrder: ["search", "rust"],
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

let exampleIndex = 0;
let animationRun = 0;
let autoRotateTimer = null;
let spinnerTimer = null;
const AUTO_ROTATE_DELAY = 5000;
const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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

window.requestAnimationFrame(() => {
  exampleIndex = 0;
  showExample(exampleIndex);
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
