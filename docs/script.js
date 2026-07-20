const examples = [
  {
    request: "jst find all files bigger than 500 mb in ~/downloads",
    result: "find ~/downloads -type f -size +500M",
  },
  {
    request: "jst show me what's using port 3000",
    result: "lsof -i :3000",
  },
  {
    request: "jst show the 10 largest files in this folder",
    result: "du -ah . | sort -hr | head -n 10",
  },
  {
    request: "jst find every TODO in rust files",
    result: "rg --glob '*.rs' TODO",
  },
  {
    request: "jst list git branches by most recent activity",
    result: "git branch --sort=-committerdate",
  },
  {
    request: "jst find files changed in the last 24 hours",
    result: "find . -type f -mtime -1",
  },
  {
    request: "jst make a receipts folder and move all PDFs into it",
    result: "mkdir -p receipts && mv -- *.pdf receipts/",
  },
  {
    request: "jst show the last 20 lines of my application log",
    result: "tail -n 20 application.log",
  },
  {
    request: "jst count the lines in every rust file",
    result: "find . -name '*.rs' -print0 | xargs -0 wc -l",
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

let exampleIndex = 0;
let animationRun = 0;

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

async function showExample(index, animate = true) {
  const example = examples[index];
  const run = ++animationRun;
  translationElement.setAttribute(
    "aria-label",
    `JST turns ${example.request} into the shell command ${example.result}`,
  );

  if (!animate || reduceMotion.matches) {
    requestElement.textContent = example.request;
    resultElement.textContent = example.result;
    resultLine.classList.add("is-visible");
    return;
  }

  requestElement.textContent = "";
  resultElement.textContent = "";
  resultLine.classList.remove("is-visible");

  await wait(220);
  const requestFinished = await typeText(requestElement, example.request, 24, run);
  if (!requestFinished || run !== animationRun) return;

  await wait(360);
  resultLine.classList.add("is-visible");
  await typeText(resultElement, example.result, 17, run);
}

anotherButton?.addEventListener("click", () => {
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

window.requestAnimationFrame(() => showExample(0));

// --- Live usage stats -------------------------------------------------

const STATS_URL = "https://jst-server.fly.dev/stats";
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

function formatDay(date) {
  const [, month, day] = date.split("-").map(Number);
  return `${MONTH_NAMES[month - 1]} ${day}`;
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
  bar.title = `${formatDay(date)} — ${formatNumber(count)} ${count === 1 ? "query" : "queries"}`;
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

  const days = stats.daily || [];
  if (days.length > 0) {
    const max = Math.max(...days.map((day) => day.count), 1);
    dayBarsElement.replaceChildren(
      ...days.map((day, index) => renderDayBar(day, index, days, max)),
    );
    document.querySelector("#day-bars-start").textContent = formatDay(days[0].date);
    document.querySelector("#day-bars-end").textContent = formatDay(days[days.length - 1].date);
  }

  statsSection.hidden = false;
  const navLink = document.querySelector("#nav-stats");
  if (navLink) navLink.hidden = false;
}

if (statsSection) {
  loadStats().catch(() => {
    // Stats are best-effort: keep the section hidden when unavailable.
  });
}
